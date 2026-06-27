---
id: "201"
phase: 2
title: Add make_process_inspector hook + TestContainerService + SC1 resume test
status: ready
depends_on: []
parallel: false
conflicts_with: ["202"]
files:
  - crates/services/src/services/container.rs
irreversible: false
scope_test: "crates/services/src/services/container.rs"
allowed_change: mixed
covers_criteria: [SC1a, SC1b, SC1c]
---
## Failing test (write first)
Add the test below to the EXISTING `#[cfg(test)] mod tests { … }` block at the bottom of
`crates/services/src/services/container.rs` (the block that already holds
`cleanup_orphan_executions_accessor_set_and_get_resume_state`). It also requires the
`TestContainerService` test double (see `## Change`, item C) in that same test module.

```rust
#[tokio::test]
async fn test_cleanup_orphan_executions_resumes_with_qa_mock_session() {
    use db::test_utils::create_test_pool;
    use db::DbMetrics;
    use executors::executors::BaseCodingAgent;
    use std::sync::Mutex;

    let (pool, _tmp) = create_test_pool().await;

    // --- seed project -> task -> task_attempt (with container_ref) ---
    let project_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES ($1, 'p', '/tmp/p')")
        .bind(project_id).execute(&pool).await.unwrap();
    let task_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO tasks (id, project_id, title, status) VALUES ($1, $2, 't', 'todo')")
        .bind(task_id).bind(project_id).execute(&pool).await.unwrap();
    let attempt_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref) VALUES ($1, $2, 'QA_MOCK', 'b', 'main', '/tmp/wt-qa')")
        .bind(attempt_id).bind(task_id).execute(&pool).await.unwrap();

    // --- stored ExecutorAction (QaMock initial request) serialized into the process row ---
    let stored = ExecutorAction::new(
        ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
            prompt: "do the task".to_string(),
            executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::QaMock),
        }),
        None,
    );
    let action_json = serde_json::to_string(&stored).unwrap();

    // --- running coding-agent process with a PID (the orphan) ---
    let process_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO execution_processes (id, task_attempt_id, run_reason, executor_action, status, pid, started_at) VALUES ($1, $2, 'codingagent', $3, 'running', 99999, datetime('now'))")
        .bind(process_id).bind(attempt_id).bind(&action_json)
        .execute(&pool).await.unwrap();

    // --- a session_id to resume into ---
    let session_row_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO executor_sessions (id, task_attempt_id, execution_process_id, session_id) VALUES ($1, $2, $3, 'sess-qa-abc')")
        .bind(session_row_id).bind(attempt_id).bind(process_id)
        .execute(&pool).await.unwrap();

    // PID 99999 is absent from the (empty) mock inspector -> fence() returns AlreadyGone.
    let service = TestContainerService {
        db: DBService { pool: pool.clone(), metrics: DbMetrics::new() },
        instance_id: "test-instance".to_string(),
        inspector: MockProcessInspector::new(),
        captured_action: Arc::new(Mutex::new(None)),
    };

    service.cleanup_orphan_executions().await.unwrap();

    // start_execution_inner must have been driven with the resume action
    let action = service.captured_action.lock().unwrap().take()
        .expect("start_execution_inner must have been called (resume path)");
    match action.typ() {
        ExecutorActionType::CodingAgentFollowUpRequest(req) => {
            assert_eq!(req.session_id, "sess-qa-abc");
            assert_eq!(req.executor_profile_id.executor, BaseCodingAgent::QaMock);
            assert_eq!(
                req.prompt,
                "Your previous session was interrupted. Please continue the task from where you left off."
            );
        }
        other => panic!("expected CodingAgentFollowUpRequest, got {:?}", other),
    }

    // resume_state recorded as 'resumed'
    let state = ExecutionProcess::get_resume_state(&pool, process_id).await.unwrap();
    assert_eq!(state, Some("resumed".to_string()));
}
```

> If `action.typ()` / `req.executor_profile_id.executor` accessor shapes differ from the above,
> match the field-access style already used by the existing `test_build_resume_action_*` tests in
> this same module (they construct and inspect `CodingAgentFollowUpRequest`). Do NOT invent new
> accessors — STOP and reconcile against those tests.

## Change
For each file in files:: (single file, three edits — A and B are production, C is test-only)

**A. Import the `ProcessInspector` trait (production).**
- **File:** `crates/services/src/services/container.rs`
- **Anchor:** the `use crate::services::{ … }` block, line ~53.
- **Before:** `    process_inspector::SysinfoProcessInspector,`
- **After:** `    process_inspector::{ProcessInspector, SysinfoProcessInspector},`

**B. Add the injection hook + route fence through it (production).**
- **Anchor 1:** the `ContainerService` trait, immediately AFTER `fn instance_id(&self) -> &str;`
  (line ~167). Add:
  ```rust
  /// Construct the ProcessInspector used by crash-recovery fencing. Production returns the
  /// real sysinfo-backed inspector; tests override this to inject a MockProcessInspector.
  fn make_process_inspector(&self) -> Box<dyn ProcessInspector> {
      Box::new(SysinfoProcessInspector::new())
  }
  ```
  (`ProcessInspector: Send + Sync` is a supertrait, so `dyn ProcessInspector` is already
  Send+Sync; the trait stays dyn-compatible — it is used as `dyn ContainerService` in `drafts.rs`.)
- **Anchor 2:** inside `cleanup_orphan_executions`, the inspector construction (line ~309).
  - **Before:** `        let inspector = SysinfoProcessInspector::new();`
  - **After:** `        let inspector = self.make_process_inspector();`
- **Anchor 3:** inside `cleanup_orphan_executions`, the fence call (line ~344).
  - **Before:** `            let fence_result = process_fence::fence(&inspector, pid_raw, &container_ref).await;`
  - **After:** `            let fence_result = process_fence::fence(inspector.as_ref(), pid_raw, &container_ref).await;`
  - (Reason: `inspector` is now `Box<dyn ProcessInspector>`; `fence<I: ProcessInspector + ?Sized>`
    takes `&I`, so pass `inspector.as_ref()` → `&dyn ProcessInspector`. `&inspector` would set
    `I = Box<…>`, which does not implement `ProcessInspector`.)

**C. Add `TestContainerService` to the `#[cfg(test)] mod tests` block (test-only).**
- **Anchor:** inside `#[cfg(test)] mod tests { use super::*; … }`, near the top of the module
  (before the test fns). Add the struct + impl. `git()`, `msg_stores()`, etc. return references;
  `unimplemented!()` type-checks there because `!` coerces to any type.
  ```rust
  use crate::services::process_inspector::MockProcessInspector;
  use std::sync::Mutex;

  struct TestContainerService {
      db: DBService,
      instance_id: String,
      inspector: MockProcessInspector,
      captured_action: Arc<Mutex<Option<ExecutorAction>>>,
  }

  #[async_trait]
  impl ContainerService for TestContainerService {
      fn db(&self) -> &DBService { &self.db }
      fn instance_id(&self) -> &str { &self.instance_id }
      fn make_process_inspector(&self) -> Box<dyn ProcessInspector> {
          Box::new(self.inspector.clone())
      }
      async fn start_execution_inner(
          &self,
          _task_attempt: &TaskAttempt,
          _execution_process: &ExecutionProcess,
          executor_action: &ExecutorAction,
      ) -> Result<(), ContainerError> {
          *self.captured_action.lock().unwrap() = Some(executor_action.clone());
          Ok(())
      }

      // --- remaining abstract methods: not exercised by cleanup_orphan_executions ---
      fn msg_stores(&self) -> &Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>> { unimplemented!() }
      fn git(&self) -> &GitService { unimplemented!() }
      fn share_publisher(&self) -> Option<&SharePublisher> { None }
      fn log_batcher(&self) -> Option<&LogBatcherHandle> { None }
      fn normalization_metrics(&self) -> &NormalizationMetrics { unimplemented!() }
      async fn store_normalization_handle(&self, _exec_id: Uuid, _handle: JoinHandle<()>) { unimplemented!() }
      async fn take_normalization_handle(&self, _exec_id: &Uuid) -> Option<JoinHandle<()>> { unimplemented!() }
      async fn get_entry_index_provider(&self, _exec_id: &Uuid) -> Option<executors::logs::utils::EntryIndexProvider> { unimplemented!() }
      async fn store_entry_index_provider(&self, _exec_id: Uuid, _provider: executors::logs::utils::EntryIndexProvider) { unimplemented!() }
      fn task_attempt_to_current_dir(&self, _task_attempt: &TaskAttempt) -> PathBuf { unimplemented!() }
      async fn create(&self, _task_attempt: &TaskAttempt) -> Result<ContainerRef, ContainerError> { unimplemented!() }
      async fn kill_all_running_processes(&self) -> Result<(), ContainerError> { unimplemented!() }
      async fn delete_inner(&self, _task_attempt: &TaskAttempt) -> Result<(), ContainerError> { unimplemented!() }
      async fn ensure_container_exists(&self, _task_attempt: &TaskAttempt) -> Result<ContainerRef, ContainerError> { unimplemented!() }
      async fn is_container_clean(&self, _task_attempt: &TaskAttempt) -> Result<bool, ContainerError> { unimplemented!() }
      async fn stop_execution(&self, _execution_process: &ExecutionProcess, _status: ExecutionProcessStatus) -> Result<(), ContainerError> { unimplemented!() }
      async fn try_commit_changes(&self, _ctx: &ExecutionContext) -> Result<bool, ContainerError> { unimplemented!() }
      async fn copy_project_files(&self, _source_dir: &Path, _target_dir: &Path, _copy_files: &str) -> Result<(), ContainerError> { unimplemented!() }
      async fn stream_diff(&self, _task_attempt: &TaskAttempt, _stats_only: bool) -> Result<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>, ContainerError> { unimplemented!() }
      async fn git_branch_prefix(&self) -> String { unimplemented!() }
  }
  ```
  > The 23 abstract methods above were copied from the trait definition. If `cargo check` reports a
  > MISSING method (trait drift since authoring) add it as `unimplemented!()`; if it reports an
  > EXTRA method (one listed here is actually a defaulted method) DELETE that stub. The compiler is
  > the source of truth for the required set — STOP only if a method that IS exercised on the resume
  > path (db / instance_id / make_process_inspector / start_execution_inner) is the one in conflict.
  - **Anchor:** the resume integration test from `## Failing test` is added to this same module.

## Allowed moves
- Only the three production edits (A, B-anchors 1/2/3) and the test-only additions (C + the test fn).
- Do NOT change `cleanup_orphan_executions` logic beyond the two inspector lines.
- Do NOT modify the `LocalContainerService` impl in `crates/local-deployment/` — the new
  `make_process_inspector` is a defaulted trait method; that impl inherits it unchanged.
- Do NOT add a new dependency (the test uses only `serde_json`, `db::test_utils`, std — all present).

## STOP triggers
- The `ContainerService` trait or `cleanup_orphan_executions` anchors are not where stated.
- `process_fence::fence` is NOT generic over `?Sized` (it is, verified — but if changed, STOP).
- Adding `make_process_inspector` breaks dyn-compatibility (`dyn ContainerService` in `drafts.rs`
  fails to compile) — STOP; do not make it generic.
- A method that IS exercised on the resume path is missing/renamed in the trait.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services test_cleanup_orphan_executions_resumes_with_qa_mock_session" bash ~/.claude/wai/scripts/task-gate.sh foundations-followup1 201` exits 0
