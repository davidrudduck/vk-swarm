---
id: "304"
phase: 3
title: Rewrite cleanup_orphan_executions to fence-then-resume (before mark-failed), incl. fallback
status: passed
depends_on: ["301", "302", "303", "104"]
parallel: false
conflicts_with: ["303"]
files:
  - crates/services/src/services/container.rs
  - crates/db/src/models/execution_process/queries.rs
irreversible: false
scope_test: "crates/services/src/services/container.rs"
allowed_change: mixed
covers_criteria: [SC1, SC8]
---
## Failing test (write first)
A recovery-classification test (in `crates/services/src/services/container.rs` `#[cfg(test)]` or a
colocated integration test) over seeded rows, proving the SC8 ordering invariant:
```rust
// Seed two running coding-agent processes on a fresh instance:
//   A) has a session_id + a resumable executor (qa_mock configured resumable) -> must be RESUMED,
//      resume_state transitions to 'resumed', and it is NEVER marked 'failed'.
//   B) has NO session_id and is non-resumable -> classified 'abandoned' and marked 'failed'.
#[tokio::test]
async fn test_recovery_resumes_resumable_and_never_marks_it_failed() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;
    let svc = test_container_service(pool.clone()).await; // existing/standard test harness
    let a = seed_running_codingagent(&pool, /*session*/ Some("sess-a"), /*resumable*/ true).await;
    let b = seed_running_codingagent(&pool, /*session*/ None, /*resumable*/ false).await;

    svc.cleanup_orphan_executions().await.unwrap();

    // SC8: resumable run was NOT failed.
    let a_after = ExecutionProcess::find_by_id(&pool, a).await.unwrap().unwrap();
    assert_ne!(a_after.status, ExecutionProcessStatus::Failed);
    assert_eq!(get_resume_state(&pool, a).await, Some("resumed".to_string()));
    // Abandoned run WAS failed.
    let b_after = ExecutionProcess::find_by_id(&pool, b).await.unwrap().unwrap();
    assert_eq!(b_after.status, ExecutionProcessStatus::Failed);
}
```
(If a full `start_execution_inner` spawn is too heavy for the unit test, assert the classification +
`resume_state` transitions + that `mark_orphaned_as_failed` was not applied to A — stub/observe the
resume call. Record the chosen seam in the ledger.)

## Change
- **File:** `crates/db/src/models/execution_process/queries.rs`
- **Anchor:** near `mark_orphaned_as_failed` (L114) and `find_running_with_pids` (L192).
- **After:** add dedicated SCALAR accessors for the resume-intent column (NOT via the `ExecutionProcess`
  FromRow struct — ledger decision): `set_resume_state(pool, id: Uuid, state: &str) -> Result<(),
  sqlx::Error>` (`UPDATE execution_processes SET resume_state = ? WHERE id = ?`) and, if needed,
  `get_resume_state(pool, id) -> Result<Option<String>, sqlx::Error>`. Optionally narrow
  `mark_orphaned_as_failed` so it does NOT touch rows already marked `resume_state IN
  ('pending','resumed')` (belt-and-braces for SC8).

- **File:** `crates/services/src/services/container.rs`
- **Anchor:** `cleanup_orphan_executions` (L239-337) — the WHOLE body is rewritten.
- **Before:** current body calls `mark_orphaned_as_failed` FIRST (L247-248), then a now-dead per-process
  loop (the `find_running` loop at L261-335 finds nothing because everything was just failed — and
  `server_instance_id` regenerates each boot, so on a real crash this fails ALL in-flight runs; the
  `InReview` flip + Hive push at L315-318 are dead-on-crash).
- **After:** rewrite to **fence-then-resume, ordered BEFORE any failure-marking** (ADR-0001 / SC8):
  1. Fetch the running coding-agent processes to consider (`find_running_with_pids`, queries.rs:192 —
     and/or `WorkstreamState::find_resumable_running` from task 104). Filter to
     `run_reason == CodingAgent`.
  2. For EACH, classify using task 301's capability map branch:
     - **fence first (always):** resolve `task_attempt.container_ref`; call `process_fence::fence(pid,
       &container_ref)` (task 302). Do NOT proceed to resume until it returns `Fenced` or `AlreadyGone`
       (never resume into a worktree with a live writer — the safety invariant).
     - **resume** (executor supports session resume AND a `session_id` exists via
       `find_latest_session_id_by_task_attempt`): set `resume_state='pending'`, call
       `self.resume_execution(&task_attempt, &process, session_id, prompt)` (task 303, prompt per 301's
       decision), set `resume_state='resumed'`. NEVER mark this row failed.
     - **cold-respawn** (no session but 301 classified the original `executor_action` safe to re-run):
       re-enter `start_execution_inner` with the ORIGINAL action; set `resume_state='resumed'`.
     - **mark-failed** (last resort: no session + non-resumable/unsafe): mark this single row failed
       (and only then propagate the existing `InReview` + `share_publisher().update_shared_task_by_id`
       outward, L315-318), set `resume_state='abandoned'`.
  3. The blanket `mark_orphaned_as_failed` (queries.rs:114) is **no longer the first action**; it is
     either removed in favour of the per-row mark-failed branch, or narrowed to truly-abandoned rows
     (rows with no recoverable state) and run AFTER the loop. It must NEVER fail a row that was resumed.

## Allowed moves
Rewrite the recovery routine and add the resume_state scalar accessors. Use the primitives from 301
(capability map), 302 (`process_fence`), 303 (`resume_execution`/`build_resume_action`), and 104
(`find_resumable_running`). Do NOT change `start_execution_inner`, the executor types, or the
share-publisher/sync plumbing (only call the existing `update_shared_task_by_id` in the mark-failed
branch exactly as today). Do NOT touch `local-deployment`.

## STOP triggers
- The single-node assumption bites: `mark_orphaned_as_failed` keys on `server_instance_id != current`,
  which also matches *other live nodes'* rows (ADR-0001 D7). For THIS workstream that is acceptable
  (single node); if the test environment has multi-instance rows, scope the recovery query to this
  node's own crashed rows and record it. Do NOT solve multi-node ownership here (that is hive-redesign).
- `resume_execution` (303) is unavailable or its signature differs → reconcile against the merged 303
  before writing (these two tasks conflict on this file; 303 lands first).
- The classification needs a capability the 301 audit marked "unknown" → take the conservative
  **mark-failed** branch and record it.

## Manual verification (the literal SC1 kill-9 smoke — record in decisions-ledger)
Using the qa_mock executor (task 201) in a dev node: start a task attempt that runs qa_mock, `kill -9`
both the mock process and the node, restart the node, and observe: the prior mock PID is fenced
(confirm dead), exactly ONE re-spawn occurs with `--resume` into the SAME `container_ref`, no second
writer, and the task is NOT shown as `failed`/`InReview`. Record the observed PIDs + resume_state.

## Done when
This task adds `set_resume_state`/`get_resume_state` (`query!`) against the `resume_state` column from
103 — the schema MUST be materialized first or the build fails (ledger Trap 2; breakdown-review R7).
**Precondition (Trap 2):** export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` to a dev DB with
the 103 migration applied (`sqlx migrate run`), so `query!` checks the LIVE schema. Do NOT
`cargo sqlx prepare` here — it churns the tracked `.sqlx` cache the gate rejects (regen is a `/wai:close`
step). With `DATABASE_URL` set, the offline cache is bypassed.

`WAI_TYPECHECK_CMD="cargo check -p services && cargo check -p db" WAI_TEST_CMD="cargo test -p services cleanup_orphan" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 304` exits 0
