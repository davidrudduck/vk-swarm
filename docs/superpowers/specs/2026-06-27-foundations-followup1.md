---
doc_type: spec
status: active
workstream: foundations-followup1
change_kind: behaviour
---

# foundations-followup1 — Close the three test coverage gaps from Phase 2a

> **Follows** [`vk-swarm-node-foundations`](./2026-06-26-vk-swarm-node-foundations.md) (Phase 2a,
> shipped 2026-06-27). Phase 2a is implementation-complete but left three documented gaps in the
> reachability gate and decisions-ledger that were explicitly scoped out. This workstream closes them.
>
> **Evidence basis:** Phase 2a decisions-ledger entries SC1 full integration test gap, D4
> (MockProcessInspector D-state gap), D6 (SC2 real-seam test gap), and the reachability gate
> verdict ("PARTIAL PASS — SC2 gap documented").

## Intent (what / why)

Phase 2a shipped three deliberate coverage gaps rather than expand scope mid-run:

1. **SC1 end-to-end crash-resume integration test** — The fence → classify → resume path through
   `cleanup_orphan_executions` has no test that exercises the *full call chain*. Individual units
   (fence, `set_resume_state`, `mark_orphaned_as_failed` guard) are tested, but no test spawns an
   execution, kills it, and verifies the resuming `ExecutorAction` is built correctly. `qa_mock` was
   forward-ported precisely to make this test possible (task 201), but the test itself was deferred.

2. **MockProcessInspector D-state mode + fence_attempt_count escalation** — `process_fence.rs`
   has a `CouldNotKill` variant for processes stuck in uninterruptible sleep (D-state), but
   `MockProcessInspector` has no stubborn-PID mode to exercise this path. The force-escalation loop
   and the `CouldNotKill` return branch are untested (see decisions-ledger 302 "Known coverage gap").
   There is also no escalation mechanism: if a process stays in D-state across multiple restart
   cycles the system silently retries indefinitely with no observable signal to the operator.

3. **SC2 boot-drain full call path test** — `test_boot_drain_includes_completed_idle_attempt`
   verifies the SQL skip-predicate via the test-private `query_drainable` helper, but never calls
   `drain_queued_messages_on_boot` or asserts that `start_queued_message_for_attempt` executes.
   The reachability gate explicitly flags this: "call path not verified; SQL predicate verified."

All three gaps are well-understood, bounded, and self-contained — closing them requires no
design changes to Phase 2a's production code, only new tests and two small supporting additions
(stubborn-PID mode in mock + `fence_attempt_count` column + escalation warning).

## Users / who is affected

- **Developers** maintaining the crash-resume and boot-drain subsystems: the gaps mean a
  regression in the critical SC1/SC2 recovery paths could go undetected until production.
- **Operators** running nodes: without the escalation warning, a process stuck in D-state after
  repeated restart attempts produces no observable signal — the only indication is a `resume_state`
  that never progresses past `'pending'`.

## Success criteria

### GAP 1 — SC1 end-to-end crash-resume integration test

**SC1a.** `cargo test -p services` includes a test that:
- Uses `qa_mock` to create a real `ExecutionProcess` row with a stored `ExecutorAction` of type
  `CodingAgentInitialRequest` (executor: QaMock) and a real `executor_sessions` row with a known
  `session_id`.
- Simulates a crash by setting `resume_state = NULL` (process never had a chance to self-report)
  and using `MockProcessInspector` with `AlreadyGone` outcome for the process PID.
- Calls `cleanup_orphan_executions` (the real trait method, against a real SQLite pool).
- Asserts that `resume_execution` is invoked with the correct `session_id` (matching the seeded
  executor session) and that the resulting `ExecutorAction` is a `CodingAgentFollowUpRequest`
  with `executor_profile_id.executor == BaseCodingAgent::QaMock`.

**SC1b.** The test does NOT require a real running process; process spawning is suppressed via a
mock or a no-op `start_execution_inner` override, or the test asserts only as far as the point
`resume_execution` would be called (verifying the classification branch, not the full spawn).

**SC1c.** `cargo test -p services <test_name>` exits 0 on CI.

### GAP 2 — MockProcessInspector stubborn-PID mode + fence_attempt_count escalation

**SC2a.** `MockProcessInspector` has a `stubborn_pids: HashSet<i64>` (or equivalent) field.
Processes in this set return "alive" on every `kill_process` call (SIGTERM and SIGKILL), causing
`process_fence::fence()` to return `FenceOutcome::CouldNotKill`.

**SC2b.** A migration adds `fence_attempt_count INTEGER NOT NULL DEFAULT 0` to
`execution_processes`. A scalar accessor (`increment_fence_attempt_count`, `get_fence_attempt_count`
or equivalent) in `crates/db/src/models/execution_process/queries.rs` increments and reads it.

**SC2c.** `cleanup_orphan_executions` increments `fence_attempt_count` each time `CouldNotKill`
is returned for a process. After reaching a configurable threshold (default: 5 attempts), it emits
a structured `tracing::warn!` including `process_id`, `fence_attempt_count`, and a human-readable
message suitable for surfacing to an operator (e.g., "process stuck in D-state after N restart
attempts — manual intervention may be required").

**SC2d.** `cargo test -p services` includes a test using stubborn-PID mode that:
- Verifies `resume_state` stays `'pending'` across multiple `CouldNotKill` cycles.
- Verifies `fence_attempt_count` increments each cycle.
- Verifies the escalation `tracing::warn!` fires when the attempt counter reaches the threshold
  (verified via a `tracing_subscriber::recorder` capture or equivalent in-process subscriber).

**SC2e.** `cargo test -p services <test_name>` exits 0 on CI.

### GAP 3 — SC2 boot-drain full call path integration test

**SC3a.** A test in `crates/local-deployment/src/container.rs` (or
`crates/local-deployment/tests/`) seeds:
- A `queued_messages` row for a known `task_attempt_id`.
- An idle/completed `execution_processes` row for that attempt (so the skip-predicate passes).
- The parent `task` and `project` rows for FK validity.

**SC3b.** The test calls `drain_queued_messages_on_boot` on a real `LocalContainerService`
instance (or a minimal test double that exposes the method without requiring full executor
infrastructure). It does NOT call `query_drainable`.

**SC3c.** The test asserts that `start_queued_message_for_attempt` was invoked for the seeded
attempt — either by observing a side effect (an `execution_processes` row created) or by
instrumenting/wrapping `start_queued_message_for_attempt`. If spawning a real executor is
infeasible in the test environment, the test may intercept at the `start_queued_message_for_attempt`
boundary and assert it was called with the correct `task_attempt_id`.

**SC3d.** `cargo test -p local-deployment <test_name>` exits 0 on CI.

### CI gate (all gaps)

All three gap-closing additions must leave the following green with zero new failures:
- `cargo clippy --all --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cd frontend && npm run lint`
- `cd frontend && npx tsc --noEmit`

## Constraints

- **No design changes to Phase 2a production code** — the gaps are in test coverage and
  observability only. Production behaviour is not modified except for: (a) the
  `fence_attempt_count` increment + escalation warning in `cleanup_orphan_executions` (Gap 2), and
  (b) the new DB column (Gap 2 migration). These are purely additive.
- **SQLx offline cache** — any new `query!`/`query_as!` macros require a `DATABASE_URL` pointing
  at a live migrated dev DB during compilation, and a `cargo sqlx prepare` pass at closeout (per
  Trap 2 in the Phase 2a decisions-ledger). Tests that use `sqlx::query()` (untyped) do not
  require cache updates.
- **MockProcessInspector is in `process_inspector/mock.rs`** — it sits at the test-seam boundary;
  the stubborn-PID mode must not affect the `SysinfoProcessInspector` production implementation.
- **`cleanup_orphan_executions` is a trait method on `ContainerService`** in
  `crates/services/src/services/container.rs` (Trap 4 in Phase 2a ledger). Integration tests for
  SC1/SC2 must call into the real trait method, not bypass it.
- **GitHub targeting** — PRs only against `davidrudduck/vk-swarm`, not `BloopAI/vibe-kanban`.
- **Safe process management** — never use `pkill`/`killall`; use `kill <PID>` or `pnpm run stop`.

## Approach

Three self-contained additions. No Phase 2a production logic is modified beyond the two
prescribed exceptions in the Constraints section; all other changes are additive.

**GAP 1** — Add a `make_process_inspector()` default method to `ContainerService` so tests can
inject `MockProcessInspector`. Update `cleanup_orphan_executions` to call it (one-line change at
line 309 of `container.rs`). Write `TestContainerService` (inside `#[cfg(test)]` in
`container.rs`) that captures `start_execution_inner` and overrides `make_process_inspector`.
The test drives the full `cleanup_orphan_executions` default method against a real SQLite pool
seeded with a running QaMock process + session row.

**GAP 2** — Add `fence_attempt_count INTEGER NOT NULL DEFAULT 0` to `execution_processes` via
migration. Add two scalar accessors (`increment_fence_attempt_count`, `get_fence_attempt_count`)
in `queries.rs`. Update the `CouldNotKill` arm of `cleanup_orphan_executions` to increment the
counter and emit a structured `tracing::warn!` at threshold. `MockProcessInspector` already has
`set_unkillable(pid)` — no mock changes needed.

**GAP 3** — Add a `#[cfg(test)]`-gated `drain_spy_tx: Option<UnboundedSender<Uuid>>` field and
a `new_for_drain_test(pool)` constructor to `LocalContainerService`. Fire the spy after the
attempt is loaded (before `start_queued_message_for_attempt` is called) so the test proves the
correct attempt was targeted without requiring full executor infrastructure. The test lives in
`crates/local-deployment/src/container.rs` alongside the existing drain tests.

## Design

Three additive code changes (one per gap) described in detail in the Implementation section below.
GAP 1 adds a `make_process_inspector` hook to `ContainerService` and a `TestContainerService` test
double. GAP 2 adds a `fence_attempt_count` DB column, scalar accessors, and CouldNotKill escalation.
GAP 3 adds a `#[cfg(test)]` drain spy and minimal test constructor to `LocalContainerService`.

## Implementation

### GAP 1 — ProcessInspector hook + TestContainerService integration test

**Production change — `crates/services/src/services/container.rs`:**

Add one default method to the `ContainerService` trait (insert after `instance_id` at ~line 168):
```rust
fn make_process_inspector(&self) -> Box<dyn ProcessInspector + Send + Sync> {
    Box::new(SysinfoProcessInspector::new())
}
```

Replace line 309 (`let inspector = SysinfoProcessInspector::new();`) with:
```rust
let inspector = self.make_process_inspector();
```

Update `fence` call at line 344 to pass `inspector.as_ref()` (or `&*inspector`) since `inspector`
is now a `Box<dyn ProcessInspector + Send + Sync>` rather than a concrete type. Verify the
`process_fence::fence` signature accepts `&dyn ProcessInspector` or generic `<I: ProcessInspector>`.

**Test struct (inside `#[cfg(test)] mod tests { ... }` at bottom of `container.rs`):**
```rust
struct TestContainerService {
    pool: SqlitePool,
    instance_id: String,
    inspector: MockProcessInspector,
    captured_action: Arc<Mutex<Option<ExecutorAction>>>,
}
```
Implement `ContainerService` for `TestContainerService`:
- `db()` — construct a `DBService` from `self.pool` (match how `LocalContainerService` wraps it)
- `instance_id()` → `&self.instance_id`
- `make_process_inspector()` → `Box::new(self.inspector.clone())`
- `start_execution_inner(_, _, action)` → clone action into `self.captured_action`, `Ok(())`
- `git()` → `unimplemented!("not called on the happy path")`
- `share_publisher()` → `None`
- All other required abstract methods → `unimplemented!("not called in cleanup test")`
- Default methods with bodies (`mark_process_failed_with_task_update`, `resume_execution`, etc.) →
  use inherited defaults (do NOT override; they chain through to `start_execution_inner`)

**Test seed (raw `sqlx::query()` against `create_test_pool()` pool, same pattern as line 1793):**
```text
project  (id=P, git_repo_path='/tmp/test')
task     (id=T, project_id=P, title='test', status='todo')
task_attempt (id=A, task_id=T, executor='QA_MOCK', branch='test-branch',
              target_branch='main', container_ref='/tmp/wt-qa-mock-A')
executor_action_json = serde_json::to_string(&ExecutorAction::new(
    ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
        prompt: "test".to_string(),
        executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::QaMock),
    }), None)).unwrap()
execution_process (id=EP, task_attempt_id=A, run_reason='codingagent',
                   executor_action=<json above>, status='running',
                   pid=99999, started_at=datetime('now'))
executor_session  (id=ES, execution_process_id=EP, session_id='sess-qa-abc')
```

`MockProcessInspector` has no processes added — PID 99999 is absent →
`process_exists(99999)` returns false → `fence()` returns `AlreadyGone`.

**Test call and assertions:**
```rust
service.cleanup_orphan_executions().await.unwrap();
let action = service.captured_action.lock().unwrap().take()
    .expect("start_execution_inner must have been called");
match action.typ() {
    ExecutorActionType::CodingAgentFollowUpRequest(req) => {
        assert_eq!(req.session_id, "sess-qa-abc");
        assert_eq!(req.executor_profile_id.executor, BaseCodingAgent::QaMock);
        assert_eq!(req.prompt,
            "Your previous session was interrupted. Please continue the task from where you left off.");
    }
    other => panic!("expected CodingAgentFollowUpRequest, got {:?}", other),
}
// Also assert DB: resume_state='resumed'
let state = ExecutionProcess::get_resume_state(&pool, process_id).await.unwrap();
assert_eq!(state, Some("resumed".to_string()));
```

**Test name:** `test_cleanup_orphan_executions_resumes_with_qa_mock_session`

**Note on `DBService` construction:** `TestContainerService::db()` must return `&DBService`.
Follow the pattern in `LocalContainerService::new()` (line ~118 in `local-deployment/src/container.rs`)
to see how `DBService` is constructed from a pool. If `DBService` has a `pub fn new(pool)` or
`pub fn from_pool(pool)`, use it. The exact constructor is intentionally not prescribed here —
it is stable and visible at the call site.

---

### GAP 2 — fence_attempt_count column + CouldNotKill escalation

**Migration — `crates/db/migrations/20260201000300_add_fence_attempt_count.sql`:**

Timestamp `20260201000300` sorts immediately after the last migration
(`20260201000200_add_workstream_state_view.sql`).

```sql
ALTER TABLE execution_processes ADD COLUMN fence_attempt_count INTEGER NOT NULL DEFAULT 0;
```

No index needed — the column is only read/written in the crash-recovery hot path, which
already isolates rows by `id`.

**New DB accessors — `crates/db/src/models/execution_process/queries.rs`** (add after
`get_resume_state`, ~line 161):

```rust
pub async fn increment_fence_attempt_count(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE execution_processes SET fence_attempt_count = fence_attempt_count + 1 WHERE id = ?",
        id
    )
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn get_fence_attempt_count(pool: &SqlitePool, id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT fence_attempt_count FROM execution_processes WHERE id = ?"#,
        id
    )
    .fetch_one(pool)
    .await
}
```

Both use `query!` / `query_scalar!` macros — they require a live migrated DB (with the new column)
for compilation (Trap 2 from Phase 2a decisions-ledger). Run `sqlx migrate run` before `cargo check`
on these files.

**Constant — `crates/services/src/services/container.rs`** (add near top of file with other
constants):
```rust
const FENCE_ESCALATION_THRESHOLD: i64 = 5;
```

**Update CouldNotKill arm in `cleanup_orphan_executions`** (replace lines 365-379):
```rust
FenceOutcome::CouldNotKill => {
    // Process survived SIGKILL (D-state / uninterruptible sleep).
    // Protect from blanket sweep by setting resume_state='pending' (already done
    // in prior cycles if this is a repeat). Increment the attempt counter.
    let _ = ExecutionProcess::set_resume_state(pool, process.id, "pending").await;
    let count = match ExecutionProcess::increment_fence_attempt_count(pool, process.id).await {
        Ok(()) => {
            ExecutionProcess::get_fence_attempt_count(pool, process.id)
                .await
                .unwrap_or(0)
        }
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

Note: `increment` + `get` are two queries. SQLite RETURNING is not universally available;
keep them separate. The double-call is safe (no concurrent writer on crash-recovery path).

**Test — `crates/services/src/services/container.rs` `#[cfg(test)]`:**

Reuse `TestContainerService` from GAP 1 with `inspector.set_unkillable(99999)` (makes
`kill_process` a no-op, so `fence()` returns `CouldNotKill`). The test needs a PID in the mock
inspector AND `set_unkillable`:
```rust
inspector.add_process(RawProcessInfo { pid: 99999, working_directory: Some("/tmp/wt-A".into()), ... });
inspector.set_unkillable(99999);
```

Test loop: call `cleanup_orphan_executions()` N times, where N = `FENCE_ESCALATION_THRESHOLD`.

Assertions after each cycle:
- `get_resume_state(pool, process_id) == Some("pending")` (never cleared by blanket)
- `get_fence_attempt_count(pool, process_id) == cycle_number` (increments each time)

After cycle N:
- Assert `get_fence_attempt_count == FENCE_ESCALATION_THRESHOLD`

For the escalation `tracing::warn!`: assert it structurally by asserting `fence_attempt_count`
reaches the threshold (the warn is inside `if count >= FENCE_ESCALATION_THRESHOLD` — reaching the
count IS the evidence). If `tracing-test` is available as a dev-dep in the services crate
(`tracing-test = "0.2"` in `[dev-dependencies]` of `crates/services/Cargo.toml`), use
`#[traced_test]` and `assert!(logs_contain("manual intervention"))`. Otherwise, the count assertion
is the primary evidence and the tracing output is verified by inspection.

**Test name:** `test_cleanup_orphan_executions_stubborn_pid_escalation`

---

### GAP 3 — drain_spy in LocalContainerService + new_for_drain_test constructor

**Production change — `crates/local-deployment/src/container.rs`:**

Add `drain_spy_tx` field and spy constructor to `LocalContainerService`:
```rust
pub struct LocalContainerService {
    // ... existing fields unchanged ...
    #[cfg(test)]
    drain_spy_tx: Option<tokio::sync::mpsc::UnboundedSender<uuid::Uuid>>,
}
```

In `LocalContainerService::new()`, initialize the field:
```rust
// At the end of the constructor body, before returning Self { ... }:
#[cfg(test)]
drain_spy_tx: None,
```

Add test-only constructor (inside `#[cfg(test)]` block or with `#[cfg(test)]` attribute):
```rust
#[cfg(test)]
pub(crate) async fn new_for_drain_test(pool: sqlx::SqlitePool) -> Self {
    use db::DBService;
    let db = DBService::new(pool.clone()); // or however DBService is constructed
    let message_queue = crate::message_queue::MessageQueueStore::new(pool.clone());
    // Construct all required fields with no-op / default values.
    // Fields not accessed by drain_queued_messages_on_boot can be stubs.
    // Follow the LocalContainerService::new() call sites for constructor details.
    Self {
        db,
        child_store: Default::default(),
        msg_stores: Default::default(),
        protocol_peers: Default::default(),
        entry_index_providers: Default::default(),
        config: Default::default(),          // Config::default() or equivalent
        git: GitService::new(),              // or stateless constructor
        image_service: ImageService::default(),
        approvals: Approvals::default(),     // or Approvals::new_auto_approve() if available
        publisher: Err(remote::RemoteClientNotConfigured),
        log_batcher: LogBatcherHandle::noop(), // or equivalent disabled handle
        message_queue,
        normalization_handles: Default::default(),
        normalization_metrics: NormalizationMetrics::default(),
        instance_id: "test-drain-instance".to_string(),
        drain_spy_tx: None,
    }
}

#[cfg(test)]
pub(crate) fn with_drain_spy(
    mut self,
    tx: tokio::sync::mpsc::UnboundedSender<uuid::Uuid>,
) -> Self {
    self.drain_spy_tx = Some(tx);
    self
}
```

**Note:** `Config::default()`, `GitService::new()`, `ImageService::default()`, `Approvals::default()`,
`LogBatcherHandle::noop()` are placeholders — the implementer looks up the exact constructors. The
goal is a `LocalContainerService` whose `db.pool` is the test pool and whose other fields do not
panic when touched by `drain_queued_messages_on_boot` before `start_queued_message_for_attempt` is
called. Fields touched by `start_queued_message_for_attempt` (git, approvals, log_batcher) do NOT
need to be real — the spy fires BEFORE that call.

**Wire the spy in `drain_queued_messages_on_boot`** (after loading `task_attempt`, before the
`start_queued_message_for_attempt` call at ~line 1396):
```rust
// After: let task_attempt = match TaskAttempt::find_by_id(pool, attempt_id) { ... };
// ... (load task) ...
// Insert spy notification BEFORE start:
#[cfg(test)]
if let Some(tx) = &self.drain_spy_tx {
    let _ = tx.send(task_attempt.id);
}
// Existing call:
if let Err(e) = self.start_queued_message_for_attempt(&task_attempt, &task).await { ... }
```

**Test — `crates/local-deployment/src/container.rs` `#[cfg(test)]`:**

Reuse the existing `seed_attempt_with_process()` helper (line ~2389) to seed the DB, then:
```rust
#[tokio::test]
async fn test_drain_queued_messages_on_boot_calls_start_for_eligible_attempt() {
    let (pool, _tmp) = create_test_pool().await;
    let attempt_id = seed_idle_attempt_with_queued_message(&pool).await;
    // ↑ helper seeds: project, task, task_attempt, completed execution_process,
    //   queued_message — reuse seed_attempt_with_process + add queued_message insert

    let (spy_tx, mut spy_rx) = tokio::sync::mpsc::unbounded_channel::<Uuid>();
    let svc = LocalContainerService::new_for_drain_test(pool.clone())
        .await
        .with_drain_spy(spy_tx);

    svc.drain_queued_messages_on_boot().await.unwrap();

    // Assert the spy was called with the seeded attempt_id
    let received = spy_rx.try_recv().expect("drain must call start for the eligible attempt");
    assert_eq!(received, attempt_id);
    // Assert no second attempt was started (no spurious drains)
    assert!(spy_rx.try_recv().is_err(), "only one attempt should be drained");
}
```

The test does NOT call `query_drainable()`. It calls `drain_queued_messages_on_boot` directly.

**Test name:** `test_drain_queued_messages_on_boot_calls_start_for_eligible_attempt`

---

## Decisions

**D1 — `make_process_inspector` hook (reversible)**
Add a default method to `ContainerService` trait so tests can inject `MockProcessInspector`.
Default behavior is unchanged (`SysinfoProcessInspector::new()`). Reversible — removing the
default method restores the original behaviour. No ADR required.

**D2 — `fence_attempt_count` column (IRREVERSIBLE — see [ADR-0005](../../dev-docs/adr/0005-fence-attempt-count-column.md))**
Store attempt count in DB so it persists across server restarts. In-memory counter was rejected
because it resets on every crash (which is exactly when D-state processes occur). SQLite column
additions are forward-only — cannot be dropped without a full table rebuild. ADR-0005 records
this constraint.

**D3 — `#[cfg(test)]` spy in `LocalContainerService` (reversible)**
Add a test-only channel field and `new_for_drain_test` constructor to `LocalContainerService`
so the drain call path can be observed without standing up full executor infrastructure. The field
is gated by `#[cfg(test)]` — it is absent from production builds. Reversible — deleting the field
and constructor restores the production struct exactly. No ADR required.

**D4 — Spy fires BEFORE `start_queued_message_for_attempt` (by design)**
The spy is sent BEFORE calling `start_queued_message_for_attempt`, not after. This is intentional:
the test verifies the CALL SITE (that drain decided to start this attempt), not the start outcome
(which would require full git/executor infrastructure). This matches SC3c's stated allowance to
"intercept at the `start_queued_message_for_attempt` boundary".

## Test strategy

All three tests are in the same crates they modify (`services` and `local-deployment`). All use
`db::test_utils::create_test_pool()` for real SQLite. No mocking of DB; all SQL runs against the
real schema.

| Test name | Crate | What it verifies |
|---|---|---|
| `test_cleanup_orphan_executions_resumes_with_qa_mock_session` | `services` | Full `cleanup_orphan_executions` path: AlreadyGone fence → session found → `resume_execution` called → `start_execution_inner` receives `CodingAgentFollowUpRequest{executor=QaMock, session_id=<seeded>}` |
| `test_cleanup_orphan_executions_stubborn_pid_escalation` | `services` | CouldNotKill cycle: `resume_state` stays `'pending'`, `fence_attempt_count` increments per cycle, value reaches `FENCE_ESCALATION_THRESHOLD` |
| `test_drain_queued_messages_on_boot_calls_start_for_eligible_attempt` | `local-deployment` | `drain_queued_messages_on_boot` identifies the correct attempt via SQL, loads it from DB, and calls the start path for it |

**Coverage summary after all three tests:**
- SC1 fence → classify → resume call path: covered end-to-end (GAP 1 test)
- SC1 D-state cycle: covered (GAP 2 test)
- SC2 boot-drain call path: covered (GAP 3 test)
- Existing Phase 2a unit tests continue to cover: `build_resume_action` variants, `set_resume_state`/`get_resume_state` accessors, SQL skip-predicate (7 existing drain tests), `fence()` outcomes (8 existing fence tests)

**CI gate:** All tests run via `cargo test --workspace`. No frontend changes → lint/tsc gates
unchanged. The `fence_attempt_count` column addition requires `cargo sqlx prepare` closeout per
Trap 2 (same pattern as Phase 2a).

## Out of scope

- Changes to the Phase 2a production recovery logic (fence, resume, boot-drain) beyond the
  additive `fence_attempt_count` escalation.
- Surfacing the escalation warning in the frontend `HiveSyncStatusCard` (a future enhancement
  that depends on this workstream shipping the column and warning first).
- Testing the `CouldNotKill` → D-state → recovery path on a real Linux process in D-state
  (infeasible without kernel cooperation; mock-based testing is sufficient and correct).
- Any work from `vk-swarm-node-ui-localize` or `vk-swarm-hive-redesign`.
- Performance improvements or refactors to Phase 2a code not required to close the gaps.
