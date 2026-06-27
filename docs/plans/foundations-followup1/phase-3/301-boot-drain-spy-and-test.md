---
id: "301"
phase: 3
title: Drain spy + new_for_drain_test + boot-drain call-path test
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/local-deployment/src/container.rs
irreversible: false
scope_test: "crates/local-deployment/src/container.rs"
allowed_change: mixed
covers_criteria: [SC3a, SC3b, SC3c, SC3d]
---
## Failing test (write first)
Add to the EXISTING `#[cfg(test)] mod tests { use super::*; … }` block in
`crates/local-deployment/src/container.rs`. This test calls `drain_queued_messages_on_boot`
directly (NOT `query_drainable`) and asserts the start path is reached for the correct attempt via a
spy channel.

> **CRITICAL — do NOT reuse `seed_attempt_with_process`.** That helper binds every UUID as
> `.to_string()` (TEXT). The REAL `drain_queued_messages_on_boot` query decodes
> `qm.task_attempt_id AS "task_attempt_id!: Uuid"`, and sqlx's `Decode for Uuid` is
> `Uuid::from_slice(value.blob())` — BLOB only (verified: `sqlx-sqlite/src/types/uuid.rs`). A
> TEXT-stored hyphenated UUID is 36 bytes → `from_slice` (needs 16) → decode error → drain returns
> Err → the test panics. (That is exactly why `query_drainable` exists with manual string parsing.)
> Seed with the production shape: bind the `Uuid` value directly (`.bind(id)`), which sqlx encodes
> as a 16-byte BLOB. The inline seed below does this.

```rust
#[tokio::test]
async fn test_drain_queued_messages_on_boot_calls_start_for_eligible_attempt() {
    let (pool, _tmp) = db::test_utils::create_test_pool().await;

    // Seed with PRODUCTION-shaped (BLOB) UUIDs so the real drain query!'s `: Uuid` decode works.
    let project_id = uuid::Uuid::new_v4();
    let task_id = uuid::Uuid::new_v4();
    let attempt_id = uuid::Uuid::new_v4();
    let process_id = uuid::Uuid::new_v4();
    let msg_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO projects (id, name, git_repo_path, created_at) VALUES (?, ?, ?, ?)")
        .bind(project_id).bind("p").bind("/tmp/r").bind(&now)
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO tasks (id, project_id, title, created_at) VALUES (?, ?, ?, ?)")
        .bind(task_id).bind(project_id).bind("t").bind(&now)
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(attempt_id).bind(task_id).bind("CLAUDE_CODE").bind("b").bind("main").bind(&now)
        .execute(&pool).await.unwrap();
    // completed + no resume_state -> passes the 3-part drain skip predicate (drainable).
    sqlx::query("INSERT INTO execution_processes (id, task_attempt_id, run_reason, executor_action, status, started_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(process_id).bind(attempt_id).bind("codingagent").bind("{}").bind("completed").bind(&now).bind(&now).bind(&now)
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO queued_messages (id, task_attempt_id, content, position, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(msg_id).bind(attempt_id).bind("hello").bind(0i64).bind(&now)
        .execute(&pool).await.unwrap();

    let (spy_tx, mut spy_rx) = tokio::sync::mpsc::unbounded_channel::<uuid::Uuid>();
    let svc = LocalContainerService::new_for_drain_test(pool.clone())
        .await
        .with_drain_spy(spy_tx);

    svc.drain_queued_messages_on_boot().await.unwrap();

    // The drain identified the attempt, loaded it, and dispatched to the start boundary.
    let received = spy_rx
        .try_recv()
        .expect("drain must reach the start path for the eligible attempt");
    assert_eq!(received, attempt_id);
    // Exactly one attempt drained (no spurious dispatch).
    assert!(spy_rx.try_recv().is_err(), "exactly one attempt should be drained");
}
```

## Change
For each file in files:: (single file — A/B/C production, D test-only)

**A. Add the `#[cfg(test)]` spy field to the struct.**
- **Anchor:** `pub struct LocalContainerService { … }`, after the `instance_id: String,` field (line ~106).
- **Before:** `    instance_id: String,\n}`
- **After:**
  ```rust
      instance_id: String,
      /// Test-only spy: when set, boot drain reports each dispatched attempt id here and
      /// skips the real start (intercept-at-boundary; see foundations-followup1 SC3c).
      #[cfg(test)]
      drain_spy_tx: Option<tokio::sync::mpsc::UnboundedSender<uuid::Uuid>>,
  }
  ```

**B. Initialise the field in `new()`.**
- **Anchor:** the `let container = LocalContainerService { … };` literal in `new()` (line ~140),
  after `instance_id,`.
- **Before:**
  ```rust
              normalization_metrics,
              instance_id,
          };
  ```
- **After:**
  ```rust
              normalization_metrics,
              instance_id,
              #[cfg(test)]
              drain_spy_tx: None,
          };
  ```

**C. Add the test constructor + spy wiring, and fire the spy in `drain_queued_messages_on_boot`.**
- **Anchor 1:** inside `impl LocalContainerService { … }`, add two `#[cfg(test)]` methods (e.g. just
  after `new()` ends, line ~161):
  ```rust
  /// Minimal constructor for boot-drain tests: builds a real LocalContainerService over the
  /// given test pool with stub-but-valid dependencies. No hive/publisher. See SC3.
  #[cfg(test)]
  pub(crate) async fn new_for_drain_test(pool: sqlx::SqlitePool) -> Self {
      let db = DBService { pool: pool.clone(), metrics: db::DbMetrics::new() };
      let msg_stores = Arc::new(RwLock::new(HashMap::new()));
      let config = Arc::new(RwLock::new(Config::default()));
      let git = GitService::new();
      let image_service = ImageService::new(pool.clone()).expect("image service for test");
      let approvals = Approvals::new(msg_stores.clone());
      let publisher = Err(RemoteClientNotConfigured);
      Self::new(db, msg_stores, config, git, image_service, approvals, publisher).await
  }

  /// Attach a boot-drain spy (test-only).
  #[cfg(test)]
  pub(crate) fn with_drain_spy(
      mut self,
      tx: tokio::sync::mpsc::UnboundedSender<uuid::Uuid>,
  ) -> Self {
      self.drain_spy_tx = Some(tx);
      self
  }
  ```
- **Anchor 2:** in `drain_queued_messages_on_boot`, immediately BEFORE the
  `if let Err(e) = self.start_queued_message_for_attempt(&task_attempt, &task).await` call (line ~1396).
  - **Before:**
    ```rust
              tracing::info!(
                  attempt_id = %attempt_id,
                  "Boot drain: starting queued message for idle attempt"
              );

              if let Err(e) = self
                  .start_queued_message_for_attempt(&task_attempt, &task)
                  .await
    ```
  - **After:**
    ```rust
              tracing::info!(
                  attempt_id = %attempt_id,
                  "Boot drain: starting queued message for idle attempt"
              );

              // Test-only: report the dispatch and skip the real start (intercept-at-boundary, SC3c).
              #[cfg(test)]
              if let Some(tx) = &self.drain_spy_tx {
                  let _ = tx.send(task_attempt.id);
                  continue;
              }

              if let Err(e) = self
                  .start_queued_message_for_attempt(&task_attempt, &task)
                  .await
    ```

**D. Add the test fn** from `## Failing test` to the same `#[cfg(test)] mod tests` block.

## Allowed moves
- Only: the struct field, its init in `new()`, the two `#[cfg(test)]` methods, the spy block in
  `drain_queued_messages_on_boot`, and the test fn.
- Do NOT change the drain SQL predicate or `start_queued_message_for_attempt`.
- Do NOT make `drain_spy_tx` a non-test field or wire it into production startup.
- All dependency constructors are already imported in this file (`DBService`,
  `RemoteClientNotConfigured`, `Approvals`, `Config`, `GitService`, `ImageService`); add only
  `db::DbMetrics` if not already in scope (use the full path `db::DbMetrics::new()`).

## STOP triggers
- `LocalContainerService::new` signature differs from
  `(db, msg_stores, config, git, image_service, approvals, publisher)` (verified against `main`).
- `Config::default()`, `GitService::new()`, `ImageService::new(pool)`, or `Approvals::new(msg_stores)`
  do not exist / have changed signatures — STOP and reconcile (do not invent constructors).
- `drain_queued_messages_on_boot` does not have the `start_queued_message_for_attempt` call at the
  stated anchor.
- The real drain query rejects the seeded UUIDs at decode — means a bind used `.to_string()` (TEXT)
  instead of the raw `Uuid` (BLOB). Re-check every `.bind(...)` uses the `Uuid` value directly.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p local-deployment" WAI_TEST_CMD="cargo test -p local-deployment test_drain_queued_messages_on_boot_calls_start_for_eligible_attempt" bash ~/.claude/wai/scripts/task-gate.sh foundations-followup1 301` exits 0
