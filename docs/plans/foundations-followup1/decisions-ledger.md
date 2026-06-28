---
topic: foundations-followup1
doc_type: decisions-ledger
---

# Decisions ledger — foundations-followup1

The executor appends per-task decisions here. Below are **pre-empted traps** (decomposer-resolved
conventions) every implementer must follow, so they never have to re-decide them. Recorded at
decompose time from the spec, ADR-0005, the Phase 2a (`vk-swarm-node-foundations`) ledger, and a
read of the live tree.

## Pre-empted traps (read before executing ANY task)

### Trap 1 — WAI gate is TypeScript-shaped; this is a Cargo workspace
Inherited from Phase 2a. The shared task-gate skips the TS type-check for non-TS repos and its
`scope_test` runner detection has **no native `cargo test` path**. Every task therefore carries BOTH
overrides INLINE in its `## Done when` line:
- `WAI_TYPECHECK_CMD="cargo check -p <crate>"`
- `WAI_TEST_CMD="cargo test -p <crate> <test_name>"`
The `scope_test:` frontmatter field names the test file (for the lint's verification check) but the
gate uses the `WAI_TEST_CMD` override at run time.

### Trap 2 — SQLx is OFFLINE here; build against a live migrated DB, do NOT `cargo sqlx prepare` in a task
Inherited from Phase 2a. This repo commits a `.sqlx` offline cache and leaves `DATABASE_URL` unset.
Tasks 101 (migration) and 102 (new `query!`/`query_scalar!` accessors) require `DATABASE_URL`
pointed at a **live migrated dev DB** at compile time:
`export DATABASE_URL=sqlite://$(pwd)/dev_assets/db.sqlite` and apply migrations
(`sqlx migrate run --source crates/db/migrations`). Do **NOT** run `cargo sqlx prepare` inside a
task — `.sqlx/*.json` are not in any task's `files:` and the gate would reject them. `.sqlx`
regeneration is a single closeout step at `/wai:close` (`cargo sqlx prepare --workspace`, committed
standalone). Tasks 201/202/301 use untyped `sqlx::query()` in tests → no cache entry needed.

### Trap 3 — `fence()` is already `?Sized`-generic; pass `inspector.as_ref()`
Verified live: `pub async fn fence<I: ProcessInspector + ?Sized>(inspector: &I, …)`
(`process_fence.rs:46`). Task 201 changes the inspector to `Box<dyn ProcessInspector>`, so the fence
call must pass `inspector.as_ref()` (→ `&dyn ProcessInspector`), NOT `&inspector` (which would set
`I = Box<…>`, and `Box<dyn ProcessInspector>` does not implement `ProcessInspector`). No change to
`fence` is needed or allowed.

### Trap 4 — recovery lives in the `services` trait; the only impl is downstream
Verified live: `cleanup_orphan_executions`, `resume_execution`, `start_execution_inner` are methods
on the `ContainerService` trait in `crates/services/src/services/container.rs`. The ONLY real impl is
`LocalContainerService` in the **downstream** `local-deployment` crate — `services` cannot depend on
it (circular). Therefore the SC1/SC2 integration tests CANNOT reuse `LocalContainerService`; task 201
writes a from-scratch `TestContainerService` in the `services` test module. Tasks 201 and 202 both
edit `container.rs` and genuinely `conflicts_with` each other (sequenced by `depends_on`: 202→201).

### Trap 5 — `MockProcessInspector` already provides the D-state seam (SC2a is pre-satisfied)
Verified live (`process_inspector/mock.rs`): the mock already `#[derive(Clone)]`s and already exposes
`set_unkillable(pid: u32)` backed by an `unkillable_pids: HashSet<u32>` — `kill_process` returns Ok
but leaves the PID in the map, so `process_exists` stays true and `fence()` returns `CouldNotKill`.
SC2a ("MockProcessInspector has a stubborn_pids field") is satisfied by this existing API; **no mock
changes are made**. Task 202's test just calls `add_process` + `set_unkillable`.

### Trap 6 — `DBService` / `LocalContainerService` test construction (no field-literal guesswork)
Verified live constructors:
- `DBService` has pub fields → construct via `DBService { pool, metrics: db::DbMetrics::new() }`
  (`db::DbMetrics` is re-exported at `db/src/lib.rs:33`; `DbMetrics::new()` is pub).
- `LocalContainerService::new(db, msg_stores, config, git, image_service, approvals, publisher)` is
  the real constructor. Task 301's `new_for_drain_test` calls it with: `Config::default()`,
  `GitService::new()`, `ImageService::new(pool).expect(...)`, `Approvals::new(msg_stores)`,
  `publisher = Err(RemoteClientNotConfigured)`. All five types are already imported in
  `local-deployment/src/container.rs`. Do NOT reconstruct the struct field-by-field — call `new()`.

### Trap 7 — GAP 3 spy intercepts AT the start boundary (SC3c), does not run the real start
The `seed_attempt_with_process` helper leaves `container_ref` NULL, so letting the real
`start_queued_message_for_attempt` run would error/panic on worktree setup. Per SC3c's explicit
allowance ("intercept at the `start_queued_message_for_attempt` boundary"), task 301's
`#[cfg(test)]` spy block sends the attempt id and `continue`s — it skips the real start. This proves
the drain selects + loads + dispatches the correct attempt (the call path), which is the SC2/D6 gap;
`start_queued_message_for_attempt`'s internals stay covered by the live `try_consume_queued_message`
path (Phase 2a D6 rationale).

### Trap 8 — `tracing-test` dev-dep for the escalation-warn assertion (SC2d); declare `Cargo.lock`
SC2d requires verifying the escalation `tracing::warn!` fires "via a subscriber capture or
equivalent". `tracing-test` is not yet a dependency. Task 202 adds `tracing-test = "0.2"` to
`crates/services/Cargo.toml` `[dev-dependencies]` and uses `#[traced_test]` + `logs_contain(...)`.
Adding the dep makes cargo regenerate the root **`Cargo.lock`** — which is why task 202 lists
`Cargo.lock` in its `files:` (the gate rejects edits to undeclared files). Let cargo write the lock
(do not hand-edit). The deterministic `fence_attempt_count == threshold` assertions are the
belt-and-suspenders backstop. If the registry is offline and the dep cannot resolve, STOP and
record — do not silently drop the warn assertion.

### Trap 9 — test UUID storage MUST be BLOB (bind the `Uuid`), not TEXT (`.to_string()`)
Verified live (`sqlx-sqlite/src/types/uuid.rs`): `impl Decode for Uuid` is
`Uuid::from_slice(value.blob())` — **BLOB only** (16 bytes). A query that projects
`AS "col!: Uuid"` (e.g. the real `drain_queued_messages_on_boot` query in
`local-deployment/src/container.rs`, and the `query!` accessors in tasks 102/201/202) will FAIL to
decode a UUID that was stored as TEXT. Production stores UUIDs as BLOB (the `uuid` feature encodes
`Uuid` as a 16-byte blob). Therefore **every test seed binds the `Uuid` value directly**
(`.bind(id)`), NOT `.bind(id.to_string())`. The pre-existing `seed_attempt_with_process` helper
binds `.to_string()` (TEXT) — it is ONLY safe with `query_drainable`'s manual string parser, and
must NOT be reused by task 301 (which drives the real `: Uuid`-decoding drain query). Task 301 seeds
inline with BLOB binds. Tasks 102/201/202 already follow the BLOB pattern (matching the working
Phase 2a `cleanup_orphan_executions_accessor_*` test).

### Sibling-advisory acknowledgement (wai-plan-lint `W:` line, SC6)
- **Migration file (101 beside `…_init.sql`)** — migrations are independent forward-only DDL, NOT a
  reimplementation of a pattern. Task 101's SQL follows house conventions confirmed against recent
  migrations (`ALTER TABLE … ADD COLUMN … DEFAULT`). Not a pattern sibling; advisory acknowledged,
  no `files:` addition needed (same disposition as Phase 2a's migration acknowledgements).

## Per-task decisions (executor appends below)

## Reachability gate

`change_kind: behaviour`. Gate is mandatory. Evidence recorded below (a)–(c).

### (a) Call-path traces (production code → changed code, cited file:line)

**GAP 1 — make_process_inspector + SC1 resume (task 201)**
- Entry: `LocalContainerService::cleanup_orphan_executions()` (inherited from `ContainerService` trait;
  called at server boot via the startup sequence that calls `cleanup_orphan_executions` on the
  deployed `LocalContainerService` instance).
- Path: line 316 `let inspector = self.make_process_inspector()` [changed from
  `SysinfoProcessInspector::new()`]; line 351 `process_fence::fence(inspector.as_ref(), …)` [changed
  from `fence(&inspector, …)`]; `FenceOutcome::AlreadyGone` branch → `build_resume_action(…)` →
  `start_execution_inner(…)`.
- Confirmed: `grep -n "make_process_inspector\|inspector.as_ref" crates/services/src/services/container.rs`
  → lines 172 (default method), 316 (call site), 351 (fence call). All three changed lines are on
  the production code path.

**GAP 2 — CouldNotKill counter + warn (task 202)**
- Entry: same `cleanup_orphan_executions()`.
- Path: `FenceOutcome::CouldNotKill` arm at ~line 382 [rewritten]: sets `resume_state='pending'`,
  calls `ExecutionProcess::increment_fence_attempt_count(pool, process.id)` and
  `get_fence_attempt_count(pool, process.id)`, emits `tracing::warn!` with "manual intervention"
  at threshold.
- Confirmed: `grep -n "FENCE_ESCALATION_THRESHOLD\|increment_fence_attempt_count\|manual intervention" crates/services/src/services/container.rs`
  → lines 129 (const), 384 (increment call), 401 (warn). All on the production CouldNotKill path.

**GAP 3 — boot-drain call path (task 301)**
- Entry: `LocalContainerService::drain_queued_messages_on_boot()` (~line 1396 in
  `crates/local-deployment/src/container.rs`); called at server boot.
- Path: real SQL predicate selects drainable attempts → per-attempt loop → spy block at line 1422
  `#[cfg(test)] if let Some(tx) = &self.drain_spy_tx { tx.send(task_attempt.id); continue; }`
  BEFORE line 1428 `if let Err(e) = self.start_queued_message_for_attempt(…)`.
- Confirmed: `grep -n "drain_spy_tx\|start_queued_message_for_attempt" crates/local-deployment/src/container.rs`
  → spy at line 1422, start call at line 1428. Spy fires first; intercept is at the real call boundary.

### (b) Real-seam tests (drive real entry points, not mocks past them)

- **SC1 test** (`test_cleanup_orphan_executions_resumes_with_qa_mock_session`): calls the REAL
  `cleanup_orphan_executions()` method via `TestContainerService` (which inherits the method from
  the trait — no stub override). The real method runs the real SQL, the real fence, and the real
  resume logic. `start_execution_inner` is the only stub (captures the action). Entry point is
  the trait method, not a helper.
- **SC2 test** (`test_cleanup_orphan_executions_stubborn_pid_escalation`): same entry point via
  `TestContainerService`. `MockProcessInspector::set_unkillable` forces `CouldNotKill` on the real
  fence path. DB counter is real (`increment_fence_attempt_count` + `get_fence_attempt_count` hit
  the real SQLite DB).
- **SC3 test** (`test_drain_queued_messages_on_boot_calls_start_for_eligible_attempt`): calls the
  REAL `LocalContainerService::drain_queued_messages_on_boot()` (not a mock). Uses a real
  SQLite DB (created by `create_test_pool`), real SQL drain predicate, and a real spy channel.
  The spy fires when the code reaches `start_queued_message_for_attempt` — the precise production
  call boundary.

### (c) Behavioural assertions (spec is test-gap closure, no historical incident)

This spec is `change_kind: behaviour` (closing Phase 2a test-coverage gaps, not fixing a
production incident). Assertions map to the SPECIFIED BEHAVIOUR that was previously untested:
- SC1: `assert_eq!(req.session_id, "sess-qa-abc")` + `assert_eq!(req.prompt, "Your previous session was interrupted…")` + `resume_state == "resumed"` — proves the end-to-end QaMock resume path (fence→classify→resume→start) works correctly.
- SC2: `assert_eq!(count, cycle)` for all cycles 1..=5 + `assert!(logs_contain("manual intervention"))` — proves the persistent D-state counter increments correctly and the escalation warn fires exactly at threshold.
- SC3: `assert_eq!(received, attempt_id)` + `spy_rx.try_recv().is_err()` — proves the drain selects the correct attempt (exactly one) and dispatches it to the start boundary.

All three (a)–(c) are satisfied. The run may close.

---

### Task 102 — gate WAI_TEST_CMD correction
Pre-existing repo issue: `cargo test -p db` (without `--lib --features test-utils`) fails to
compile the integration test `crates/db/tests/task_visibility_discriminator.rs`, which uses
`db::test_utils::create_test_pool` — a module gated by `#[cfg(any(test, feature = "test-utils"))]`.
Integration tests compile `db` as an external crate, so `#[cfg(test)]` on the library does not
apply; they need the `test-utils` feature. This failure exists BEFORE task 102 (verified by
git stash to pre-102 state). The gate was run with the corrected override:
  `WAI_TEST_CMD="cargo test -p db --lib --features test-utils fence_attempt_count_increments_and_reads_back"`
Future tasks targeting `crates/db --lib` tests should use this form.
