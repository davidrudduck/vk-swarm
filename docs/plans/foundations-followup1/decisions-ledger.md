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
