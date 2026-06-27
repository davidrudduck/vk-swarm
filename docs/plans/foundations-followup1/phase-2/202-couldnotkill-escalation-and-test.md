---
id: "202"
phase: 2
title: CouldNotKill escalation (counter + warn) + stubborn-PID escalation test
status: ready
depends_on: ["102", "201"]
parallel: false
conflicts_with: ["201"]
files:
  - crates/services/src/services/container.rs
  - crates/services/Cargo.toml
  - Cargo.lock
irreversible: false
scope_test: "crates/services/src/services/container.rs"
allowed_change: mixed
covers_criteria: [SC2a, SC2c, SC2d, SC2e]
---
## Failing test (write first)
Add to the `#[cfg(test)] mod tests` block in `crates/services/src/services/container.rs`, reusing
the `TestContainerService` test double introduced by task 201. This test exercises the
`MockProcessInspector::set_unkillable` D-state path (SC2a is satisfied by that existing mock API).

```rust
#[tokio::test]
#[traced_test]
async fn test_cleanup_orphan_executions_stubborn_pid_escalation() {
    use db::test_utils::create_test_pool;
    use db::DbMetrics;
    use crate::services::process_inspector::RawProcessInfo;
    use std::sync::Mutex;

    let (pool, _tmp) = create_test_pool().await;

    let project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES ($1, 'p', '/tmp/p')")
        .bind(project_id).execute(&pool).await.unwrap();
    let task_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO tasks (id, project_id, title, status) VALUES ($1, $2, 't', 'todo')")
        .bind(task_id).bind(project_id).execute(&pool).await.unwrap();
    let attempt_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref) VALUES ($1, $2, 'QA_MOCK', 'b', 'main', '/tmp/wt-stuck')")
        .bind(attempt_id).bind(task_id).execute(&pool).await.unwrap();
    let process_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO execution_processes (id, task_attempt_id, run_reason, executor_action, status, pid, started_at) VALUES ($1, $2, 'codingagent', '{}', 'running', 4242, datetime('now'))")
        .bind(process_id).bind(attempt_id).execute(&pool).await.unwrap();

    // Inspector: pid 4242 lives under the worktree marker AND is unkillable -> fence() = CouldNotKill.
    let inspector = MockProcessInspector::new();
    inspector.add_process(RawProcessInfo {
        pid: 4242,
        parent_pid: None,
        name: "qa".to_string(),
        command: vec![],
        working_directory: Some("/tmp/wt-stuck".to_string()),
        memory_bytes: 0,
        cpu_percent: 0.0,
    });
    inspector.set_unkillable(4242);

    let service = TestContainerService {
        db: DBService { pool: pool.clone(), metrics: DbMetrics::new() },
        instance_id: "test-instance".to_string(),
        inspector,
        captured_action: Arc::new(Mutex::new(None)),
    };

    // Run recovery FENCE_ESCALATION_THRESHOLD times; each cycle is a CouldNotKill.
    for cycle in 1..=FENCE_ESCALATION_THRESHOLD {
        service.cleanup_orphan_executions().await.unwrap();
        // resume_state stays 'pending' (never marked failed, never resumed)
        let state = ExecutionProcess::get_resume_state(&pool, process_id).await.unwrap();
        assert_eq!(state, Some("pending".to_string()), "cycle {cycle}: must stay pending");
        // counter increments each cycle
        let count = ExecutionProcess::get_fence_attempt_count(&pool, process_id).await.unwrap();
        assert_eq!(count, cycle, "cycle {cycle}: fence_attempt_count");
    }

    // never resumed (start_execution_inner not called)
    assert!(service.captured_action.lock().unwrap().is_none());
    // escalation warning fired at the threshold
    assert!(logs_contain("manual intervention"));
    // process row is still running (CouldNotKill must NOT mark it failed — SC1)
    let proc = ExecutionProcess::find_by_id(&pool, process_id).await.unwrap().unwrap();
    assert_eq!(proc.status, ExecutionProcessStatus::Running);
}
```

> `RawProcessInfo`'s exact fields are SEVEN: `pid: u32`, `parent_pid: Option<u32>`, `name: String`,
> `command: Vec<String>`, `working_directory: Option<String>`, `memory_bytes: u64`,
> `cpu_percent: f32` (verified `process_inspector/mod.rs:33-48`). All seven MUST be named in the
> literal (no `..Default::default()`). Alternatively use the constructor
> `RawProcessInfo::new(4242, None, "qa".to_string(), vec![], Some("/tmp/wt-stuck".to_string()), 0, 0.0)`.

## Change
For each file in files::

**A. Add the threshold constant (`crates/services/src/services/container.rs`).**
- **Anchor:** module level, just BEFORE `#[async_trait]\npub trait ContainerService {` (line ~127),
  after the free `build_resume_action` helper / other module items.
- **Before:** (no such const)
- **After:**
  ```rust
  /// Number of consecutive CouldNotKill fence cycles after which crash-recovery emits an
  /// operator-facing escalation warning for a process stuck in D-state. See ADR-0005.
  const FENCE_ESCALATION_THRESHOLD: i64 = 5;
  ```

**B. Rewrite the CouldNotKill arm of `cleanup_orphan_executions` (same file, line ~365).**
- **Anchor:** the `FenceOutcome::CouldNotKill => { … }` match arm inside `cleanup_orphan_executions`.
- **Before:**
  ```rust
                  FenceOutcome::CouldNotKill => {
                      // Process survived SIGKILL (D-state / uninterruptible sleep).
                      // Resuming into a potentially live writer violates SC1; skip this process.
                      // Set resume_state='pending' so the blanket mark_orphaned_as_failed guard
                      // (which excludes 'pending' and 'resumed') does NOT mark this row failed —
                      // the process is still alive and will be fenced again on next restart.
                      let _ =
                          ExecutionProcess::set_resume_state(pool, process.id, "pending").await;
                      tracing::warn!(
                          process_id = %process.id,
                          pid = pid_raw,
                          "Process survived SIGKILL (D-state); skipping recovery to avoid concurrent writer"
                      );
                      continue;
                  }
  ```
- **After:**
  ```rust
                  FenceOutcome::CouldNotKill => {
                      // Process survived SIGKILL (D-state / uninterruptible sleep).
                      // Resuming into a potentially live writer violates SC1; skip this process.
                      // Set resume_state='pending' so the blanket mark_orphaned_as_failed guard
                      // (which excludes 'pending' and 'resumed') does NOT mark this row failed —
                      // the process is still alive and will be fenced again on next restart.
                      let _ =
                          ExecutionProcess::set_resume_state(pool, process.id, "pending").await;
                      let count = match ExecutionProcess::increment_fence_attempt_count(
                          pool, process.id,
                      )
                      .await
                      {
                          Ok(()) => ExecutionProcess::get_fence_attempt_count(pool, process.id)
                              .await
                              .unwrap_or(0),
                          Err(e) => {
                              tracing::error!(
                                  process_id = %process.id,
                                  error = ?e,
                                  "Failed to increment fence_attempt_count; cannot escalate"
                              );
                              continue;
                          }
                      };
                      if count >= FENCE_ESCALATION_THRESHOLD {
                          tracing::warn!(
                              process_id = %process.id,
                              pid = pid_raw,
                              fence_attempt_count = count,
                              "Process stuck in D-state after {} restart attempts — manual intervention may be required",
                              count
                          );
                      } else {
                          tracing::warn!(
                              process_id = %process.id,
                              pid = pid_raw,
                              "Process survived SIGKILL (D-state); skipping recovery to avoid concurrent writer"
                          );
                      }
                      continue;
                  }
  ```

**C. Add the `tracing-test` dev-dependency (`crates/services/Cargo.toml`).**
- **Anchor:** the `[dev-dependencies]` table (currently `wiremock`, `serial_test`, `db`).
- **Before:**
  ```toml
  [dev-dependencies]
  wiremock = "0.6"
  ```
- **After:**
  ```toml
  [dev-dependencies]
  tracing-test = "0.2"
  wiremock = "0.6"
  ```
- Then add `use tracing_test::traced_test;` inside the `#[cfg(test)] mod tests` block's imports
  (next to the other `use` lines), so `#[traced_test]` resolves.

**D. Update `Cargo.lock` (root).**
- Adding the dev-dep makes cargo resolve `tracing-test` (+ its small `tracing-subscriber`-based
  tree, most of which is already locked transitively). Running `cargo check -p services` (or
  `cargo test`) regenerates `Cargo.lock` automatically — let cargo write it; do NOT hand-edit.
  `Cargo.lock` is listed in `files:` so the gate accepts the churn.

## Allowed moves
- Only the const, the CouldNotKill arm rewrite, the Cargo.toml dev-dep, the cargo-regenerated
  `Cargo.lock` delta, the `traced_test` import, and the new test fn.
- Do NOT touch any other match arm (AlreadyGone / Fenced / NotOurProcess) or the resume logic.
- Do NOT run `cargo sqlx prepare` (no new `query!` here; the accessors landed in task 102).

## STOP triggers
- The `FenceOutcome::CouldNotKill` arm text does not match the Before block (task 201 or a prior
  change moved it) — re-locate by the `FenceOutcome::CouldNotKill =>` match label, do not guess.
- `increment_fence_attempt_count` / `get_fence_attempt_count` are absent → task 102 not landed.
- `tracing-test` fails to resolve from the registry (offline) → STOP and report; do NOT silently
  drop the `logs_contain` assertion (the count assertions remain valid but SC2d wants the warn
  verified). Record the blocker in the decisions-ledger.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services test_cleanup_orphan_executions_stubborn_pid_escalation" bash ~/.claude/wai/scripts/task-gate.sh foundations-followup1 202` exits 0
