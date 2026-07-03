# Post-Phase Integrated Adversarial Review — Phase 4 (inbound collapse, tasks 401–405)

Reviewer: opencode (glm-5.2) — cross-model challenger
Scope: full phase diff `3769a874..e34b8797`
Date: 2026-07-03

## Ground-truth command outputs (cited in findings below)

`git diff 3769a874..e34b8797 --stat` →
`crates/db/src/models/node_outbox.rs                |  21 ++`
`crates/db/src/models/task/sync.rs                  | 319 +++++++++++++++++`
`crates/services/src/services/electric_task_sync.rs | 384 ---------------------`
`crates/services/src/services/mod.rs                |   4 +-`
`crates/services/src/services/node_runner.rs        |  21 +-`
`crates/services/src/services/share.rs              |   4 -`
`crates/services/src/services/share/processor.rs    |  20 +-`

`cargo check -p db` → `Finished `dev` profile ... in 3.73s`
`cargo check -p services` → `Finished `dev` profile ... in 6.11s`
`cargo check -p services --all-targets` → `Finished `dev` profile ... in 27.23s`

`cargo test -p db --lib unlink_` → `3 passed; 0 failed`
`cargo test -p db --lib upsert_remote_task_` → `2 passed; 0 failed`
`cargo test -p db --lib ts5_one_delete_outcome_both_legs` → `1 passed; 0 failed`

`git grep -nF electric_task_sync -- crates/ ':!docs/'` → exit 1 (ZERO hits)
`git grep -nF ElectricTaskSyncService -- crates/` → exit 1 (ZERO hits)
`git grep -nF sync_project_tasks -- crates/` → exit 1 (ZERO hits)
`git status --porcelain crates/db/.sqlx/` → empty (no untracked)

## Findings

### F1 — CONFORMS (hunt 1: 402+403 dirty-guard vs unlink)
`unlink_by_shared_task_id` is a DIRECT SQL `UPDATE tasks SET shared_task_id = NULL WHERE shared_task_id = ?` (sync.rs diff, +`pub async fn unlink_by_shared_task_id` body) — does NOT go through `upsert_remote_task`, so the 403 dirty-guard (sync.rs:271 `if let Some(existing) = Task::find_by_shared_task_id ... && ...::has_unacked_for_entity`) never fires on a delete. Per ADR-0007 this is CORRECT: a hive soft-delete clears the link regardless of a pending local op; the local row + attempt are retained, the op acks normally. No bug. (Tried to disprove by checking if the WS deleted leg calls upsert — it does not: processor.rs diff routes `task.deleted` → `process_task_deleted_event` → `Task::unlink_by_shared_task_id(tx.as_mut(), ...)`.)

### F2 — CONFORMS (hunt 2: 402+404 reassigned shared_id change)
`process_task_upsert_event` (processor.rs:379) calls `upsert_remote_task(&self.db.pool, ..., hive_task.id, ...)` keyed on `hive_task.id` (the NEW shared_id). `upsert_remote_task` (sync.rs:271 guard, :297 `ON CONFLICT(shared_task_id)`) finds/updates by the inbound shared_id. A reassignment that changes the shared_id would create a NEW row (old row keeps its old link until a separate `task.deleted` for the old id arrives) — which is the hive's responsibility, not a node bug. Reassignment within the same shared_id (the common case: assignee field changes, id stable) updates in place. The dirty-guard (403) fires only if the SAME shared_id has an unacked op, which is the intended skip. No cross-task bug.

### F3 — DEVIATES (hunt 3: orphaned dead helpers + test file)
`git grep -nF delete_by_shared_task_id -- crates/`:
`crates/db/src/models/task/sync.rs:372:    pub async fn delete_by_shared_task_id<'e, E>(` (definition)
`crates/db/src/models/task/sync.rs:1395:    async fn test_delete_by_shared_task_id() {` (self-test)
`crates/services/tests/electric_task_sync.rs:512:    Task::delete_by_shared_task_id(&pool, shared_task_id)` (CALLER)
`crates/services/tests/electric_task_sync.rs:650:    Task::delete_by_shared_task_id(&pool, id).await.unwrap();` (CALLER)
`crates/services/tests/electric_task_sync.rs:728:    Task::delete_by_shared_task_id(&pool, id).await.unwrap();` (CALLER)
`git grep -nF delete_stale_shared_tasks -- crates/`:
`crates/db/src/models/task/sync.rs:393:    pub async fn delete_stale_shared_tasks(` (definition — NO non-test callers)

After 405 deleted `electric_task_sync.rs`, the ONLY remaining callers of `delete_by_shared_task_id` are in `crates/services/tests/electric_task_sync.rs`, and `delete_stale_shared_tasks` has ZERO non-test callers anywhere. The helpers are orphaned production code (only their own unit tests + a test-file referencing the now-deleted service keep them referenced). 402 explicitly forbade touching them ("405 owns dead-code deletion if they become unused") but 405's `files:` scope listed ONLY `electric_task_sync.rs`, `mod.rs`, `share.rs` — NOT `crates/services/tests/electric_task_sync.rs` and NOT `crates/db/src/models/task/sync.rs`. So neither task owned removing them, and both were left as dead production code.

**Fix:** In the same session (No-Deferred-Remediation), delete the orphaned helpers `Task::delete_by_shared_task_id` (sync.rs:372) and `Task::delete_stale_shared_tasks` (sync.rs:393) PLUS their unit tests (`test_delete_by_shared_task_id` sync.rs:1395; the `test_delete_stale_shared_tasks_*` tests in `crates/db/tests/bulk_operations.rs:213/249/276/316`) AND `git rm crates/services/tests/electric_task_sync.rs` (the whole file is a test for the deleted service; its filename also violates the 405 `forbid_after: ["electric_task_sync"]` token and the `ElectricTaskSync` doc-comment on its line 1 violates `forbid_after: ["ElectricTaskSyncService"]`'s sibling — see F5). Record in the decisions-ledger under Task 405 that the helpers + test file were confirmed orphaned by this integrated review and removed. Re-run `cargo check -p services --all-targets` + `cargo test --workspace` to confirm green.

### F4 — DEVIATES (hunt 6: 404 test gap; 401 claims coverage that does not exist)
401 task spec `docs/plans/vk-swarm-hive-redesign/phase-4/401-single-live-channel-guard.md:99-102` states:
`> The dirty-guard (assertion 2) and `task.reassigned` (assertion 3) are proven by 403's`
`> `upsert_remote_task_skips_when_local_op_unacked` and 404's `process_event_applies_task_reassigned``
`> respectively — TS5 is covered by THIS task's claim plus those tests`

`git grep -rn "process_event_applies_task_reassigned\|task_reassigned" -- crates/services/` → ZERO hits (only `processor.rs:68/71` comment + arm, no test).
`404 task spec:32:    async fn process_event_applies_task_reassigned() {` — the test was AUTHORED in the spec but NEVER written.
`docs/plans/.../decisions-ledger.md:1399: Used `## Manual verification` fallback (no unit-test harness exists for the share` — 404 used the manual-verification fallback and recorded it.

403's dirty-guard test DOES exist and passes: `upsert_remote_task_skips_when_local_op_unacked ... ok` (per `cargo test -p db --lib upsert_remote_task_`). So TS5 assertion 2 is covered. But TS5 assertion 3 (`task.reassigned` is APPLIED, not dropped) has NO automated test anywhere — 404's manual verification only proves the string sits in the match arm (compile-time), not that routing works at runtime. 401 claims `covers_tests: [TS5]` via 404's non-existent test. This is a cross-task coverage gap the per-task panels could not see: 404's panel verified the arm edit (in-isolation correct); 401's panel verified 401's own test + the spec's coverage CLAIM (which rested on 404's test existing).

**Fix:** Author the `process_event_applies_task_reassigned` test the 404 spec dictated. The 404 spec's own `## Manual verification` says "PREFER the unit test — this fallback is only if no harness exists." A harness CAN be built without inventing a new pattern: `crates/services/tests/` already has integration tests (e.g. `electric_task_sync.rs` used `setup_db()` + `wiremock`); build a `#[tokio::test]` in `crates/services/tests/task_reassigned.rs` (or a `mod tests` in `processor.rs`) that constructs an `ActivityProcessor` over a hermetic `SqlitePool`, links a local project + task, fires a `task.reassigned` `ActivityEvent`, and asserts `remote_assignee_user_id` updated + `remote_version` advanced. Then update 401's spec comment (401-single-live-channel-guard.md:99-102) to reference the now-real test, and re-run `cargo test --workspace`. Record in the decisions-ledger under Task 404 that the manual-verification fallback was superseded by the real test in this integrated review.

### F5 — CONFORMS (hunt 9: forbid_after integrity)
`git grep -nF electric_task_sync -- crates/ ':!docs/'` → exit 1 (ZERO content hits). `git grep -nF ElectricTaskSyncService -- crates/` → exit 1. `git grep -nF sync_project_tasks -- crates/` → exit 1. The three forbid_after tokens are absent from crates/ CONTENT. (The PATH `crates/services/tests/electric_task_sync.rs` remains and its line-1 doc comment has `ElectricTaskSync` — but that token is NOT in the forbid_after list; `ElectricTaskSyncService` is, and that is absent. See F3 for the orphaned-test-file cleanup recommendation, which is a dead-code issue, not a forbid_after violation.)

### F6 — CONFORMS (hunt 5: 403 dirty-guard test exists)
`grep -n "upsert_remote_task_skips_when_local_op_unacked" crates/db/src/models/task/sync.rs` → present; `cargo test -p db --lib upsert_remote_task_` → `upsert_remote_task_skips_when_local_op_unacked ... ok`. The test enqueues an unacked op then re-upserts and asserts `after.title == "remote-title"` + `after.remote_version == 1` + `returned.title == "remote-title"` — genuinely exercises the guard.

### F7 — CONFORMS (hunt 4: 401 both-legs test)
`ts5_one_delete_outcome_both_legs_attempt_retained` drives `unlink_by_shared_task_id` through BOTH `&pool` (LEG A) and `tx.as_mut()` (LEG B), then asserts on BOTH legs: row retained, `shared_task_id` cleared, `attempts.len() == 1`. This verifies the helper's executor-generic signature (the actual seam the WS leg uses). It does NOT drive `process_task_deleted_event` end-to-end — but the processor.rs diff shows the WS leg is now a thin `Task::unlink_by_shared_task_id(tx.as_mut(), hive_task.id)` call (the old `find_by_shared_task_id` + `set_shared_task_id(NULL)` no-op bug is gone), so the helper IS the leg. Sufficient for TS5 assertion 1.

### F8 — CONFORMS (hunts 7, 8, 10: compile, tests, sqlx cache)
All green per the command outputs above. No untracked `.sqlx` files.

## VERDICT: DEVIATES

Two cross-task deviations, both fixable in-session per the No-Deferred-Remediation rule:

1. **F3 (dead helpers + orphaned test file):** `Task::delete_by_shared_task_id`, `Task::delete_stale_shared_tasks`, their unit tests, and `crates/services/tests/electric_task_sync.rs` are orphaned by 405's deletion but neither 402 nor 405 owned removing them. Fix: delete all of the above; re-run `cargo check -p services --all-targets` + `cargo test --workspace`.

2. **F4 (TS5 assertion 3 untested):** 401 claims `covers_tests: [TS5]` resting on 404's `process_event_applies_task_reassigned` test, which 404 never wrote (manual-verification fallback). Fix: author the test the 404 spec dictated; update 401's spec comment; re-run `cargo test --workspace`.

Both fixes must be recorded in the decisions-ledger under their respective tasks before the PR is pushed.