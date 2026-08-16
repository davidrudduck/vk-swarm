---
id: "022"
phase: 3
title: "Emit task_created / task_status_changed from Task::upsert_remote_task"
status: ready
depends_on: ["006"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/task/sync.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: ["SC1"]
covers_tests: ["TS3"]
---
## Why this site (context, not action)
`Task::upsert_remote_task` (`crates/db/src/models/task/sync.rs:253`) is the single LIVE write path
for remote-originated task lifecycle: its production callers are the remote-task route handlers
(`crates/server/src/routes/tasks/handlers/remote.rs:82/165/369`, `status.rs:62/129/393` — which
include USER-DRIVEN status changes on remote-project tasks), the share activity processor
(`crates/services/src/services/share/processor.rs:379`), and the node_runner reconcile leg
(`crates/services/src/services/node_runner.rs:1361`). Today none of those changes journal anything —
an SC6 `task_status_changed` hook never fires for them. Caller signatures do NOT change: the
function keeps taking `pool: &SqlitePool`.

Do NOT touch the other three lifecycle writes in this file. `sync_from_shared_task` (`:20`) has
zero callers; `delete_by_shared_task_id` (`:375`) and `delete_stale_shared_tasks` (`:396`) have
test-only callers (ADR-0007 soft-unlink replaced hard delete). They are allowlisted dead code
handled by task 021, and their removal is a tracked backlog finding — not your problem here.

## Failing test (write first)
**File:** `crates/db/src/models/task/sync.rs` — a NEW `#[cfg(test)] mod sync_event_tests` at the
END of the file, after (not inside) the existing `#[cfg(test)]` module that starts at `:880`,
mirroring task 006's colocated-emission-test convention. Use `db::test_utils::create_test_pool()` — never hand-written
`CREATE TABLE`. Assert journal rows by filtering `event_journal` on `event_type` — NEVER
`rows.is_empty()`-style assertions. All state asserted on is COMMITTED state (the function commits
internally), so the vacuous-test trap does not arise; do not add rollback choreography.

1. `upsert_insert_emits_task_created` — call `upsert_remote_task` with a fresh `shared_task_id`;
   assert exactly one `task_created` row exists whose payload `task_id`/`project_id` match the
   returned task.
2. `upsert_status_change_emits_task_status_changed` — upsert once (status `todo`), upsert again
   with `remote_version + 1` and status `inprogress`; assert exactly one `task_status_changed` row,
   payload `old_status` = `todo`, `new_status` = `inprogress`. This is the test that fails if the
   old status is read outside the write transaction or not read at all.
3. `upsert_without_status_change_emits_nothing` — second upsert with `remote_version + 1`, a NEW
   title, and the SAME status; assert the title change applied (returned task) AND the
   `task_status_changed` count is unchanged and no new `task_created` row exists.
4. `version_stale_upsert_emits_nothing` — second upsert with the SAME `remote_version` (the
   `WHERE excluded.remote_version > tasks.remote_version` arm skips the update); assert the journal
   gained no rows of either type and the function still returns the existing task.
5. `dirty_guard_skip_emits_nothing` — create the linked task, enqueue an unacked outbox op for it
   (use the same repository call the guard checks:
   `OutboxRepository::has_unacked_for_entity` — seed via the outbox repository's enqueue used in
   existing outbox tests), then upsert; assert the early return fired (returned task equals the
   retained local row) and the journal gained no rows.

## Change
**The discrimination problem, and the dictated shape.** Emitting `task_created` vs
`task_status_changed` requires knowing whether a row existed and its OLD status. You may NOT learn
that with a SELECT at the start of a transaction: a deferred transaction whose first statement is a
read takes a read snapshot, and the later UPDATE upgrade earns `SQLITE_BUSY_SNAPSHOT` (517) under
WAL, which the busy handler does NOT retry — this exact shape has caused real failures twice in
this workstream (`Task::delete` pool path, `mark_orphaned_as_failed`). You may also NOT reuse the
dirty-guard's pool read as the old status — it is outside the transaction and races concurrent
writers. The probe below is a WRITE, so the transaction is a write transaction from its first
statement, and SQLite's single-writer rule makes the probe→upsert pair race-free.

Restructure the function body AFTER the dirty-guard early return (leave the guard exactly as it
is, reads on the pool, `:271-279`):

1. `let mut tx = pool.begin().await?;`
2. **Probe (first statement, a self-assignment write):**
   ```sql
   UPDATE tasks SET remote_version = remote_version
   WHERE shared_task_id = $1
   RETURNING id, status
   ```
   fetched optional as `(Uuid, TaskStatus)`. `shared_task_id` has a unique partial index (it is the
   upsert's `ON CONFLICT` target), so at most one row matches. `None` = no existing row.
3. **The existing `INSERT ... ON CONFLICT ... RETURNING` statement, textually unchanged**, executed
   on `&mut *tx` instead of `pool` (`.fetch_optional(&mut *tx)`).
4. **Emission, all four cases dictated:**
   - probe `None`, upsert returned `Some(task)` → append
     `NodeEvent::TaskCreated { task_id: task.id, project_id: task.project_id }`.
   - probe `Some((_, old_status))`, upsert returned `Some(task)`, `task.status != old_status` →
     append `NodeEvent::TaskStatusChanged { task_id: task.id, old_status, new_status: task.status }`.
   - probe `Some`, upsert returned `Some`, status equal → append NOTHING.
   - upsert returned `None` (version-stale skip) → append NOTHING.
   Append via `event_journal::append(&mut *tx, &event)` — the same call idiom as
   `crates/db/src/models/task/queries.rs:315/384/495`. Propagate append errors with `?` exactly as
   006's sites do (the append shares the transaction; a failed append rolls the write back — that
   is the D2 contract, unlike the connectivity sites which have no accompanying write).
5. `tx.commit().await?;`
6. The existing stale-skip fallback (`Task::find_by_shared_task_id(...)` when the upsert returned
   `None`) runs AFTER the commit, on the pool, unchanged in content.

**Sibling alignment:** before writing, read `Task::update` and `Task::update_status`
(`crates/db/src/models/task/queries.rs:351` region, `crates/db/src/models/task/hierarchy.rs:34-80`)
— the "exactly one event, only on an actual change, old status read inside the write transaction"
convention this task mirrors. Any divergence goes in the ledger.

**Error type note:** `event_journal::append` returns `Result<i64, EventJournalError>`;
`upsert_remote_task` returns `Result<Self, sqlx::Error>`. Convert exactly the way 006 does at its
three append sites in `queries.rs` — read one of them and copy the idiom; if 006 used a From/map
that does not compose here, STOP rather than inventing a new conversion.

## Attempt 2 corrections (2026-08-16 — both panels REJECT; all dictated)
Attempt 1 (commit `feba43c4`) got the four-case emission table, the tests, and the transaction
ordering right — panel B killed 7/7 executed mutations against them. Two things change:

**1. THE PROBE MECHANISM IS REPLACED (panel A BLOCKING — a defect in this task file's own
design, not the implementer's).** The self-assignment `UPDATE ... SET remote_version =
remote_version` premise was "a self-assignment write is invisible". FALSE in this codebase: change
notification is not a SQL trigger, it is a SQLite UPDATE HOOK installed on every production pool
connection (`crates/services/src/services/events.rs:153`, wired at
`crates/local-deployment/src/lib.rs:158-166`), and `HookTables` includes `tasks`
(`events/types.rs:26-28`). Panel A measured it: the probe fires `tasks/Update` → a false SSE
record-patch on the version-stale path (zero → one per stale upsert; N per hive reconnect via the
reconcile loop) and a doubled patch on every applying path, each with a `find_by_rowid` pool
round-trip. It also left a committed row-write behind on an error path that previously wrote
nothing.

Replace the probe as follows, exactly:
- `let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;` — `Pool::begin_with` exists in this
  workspace's sqlx (sqlx-core-0.8.6 `src/pool/mod.rs:391`) and takes the BEGIN statement verbatim.
  `BEGIN IMMEDIATE` acquires SQLite's RESERVED (write) lock AT BEGIN, so the transaction never
  holds a read snapshot it must upgrade — the SQLITE_BUSY_SNAPSHOT (517) class this task guards
  against is structurally gone, and contention at BEGIN surfaces as plain SQLITE_BUSY, which the
  pool's 30s busy_timeout (`crates/db/src/lib.rs:49`) retries.
- First statement: a plain read probe,
  `sqlx::query_as::<_, (Uuid, TaskStatus)>("SELECT id, status FROM tasks WHERE shared_task_id = ?")`
  on `&mut *tx`. Under IMMEDIATE this is safe where a deferred SELECT-first was not. No row write,
  no hook firing, no journal noise, nothing committed on error paths.
- Everything downstream (upsert on `&mut *tx`, four-case emission, commit, fallback) is unchanged
  from attempt 1.

**2. TEST 6 IS ADDED (panel A's shippable recommendation):**
`concurrent_upserts_serialize_without_errors` — spawn N=16 concurrent tasks (multi-thread
runtime) all upserting the SAME `shared_task_id` with distinct `remote_version`s 1..=16 through
the test pool; join all; assert ZERO errors and exactly ONE `task_created` row. This is the only
test that catches a future regression reintroducing a read-first deferred transaction — the
defect class that has now bitten this workstream three times.

**3. LEDGER CORRECTIONS (panel B BLOCKINGs + minors — append-only, never edit old lines).**
Append a correction section stating: (a) attempt 1's item-2 rationale ("distinct event types")
was FALSE both ways — `Task::create` and upsert-insert both emit `task_created`; the true reason
tests 1-3 are baseline-safe is that their setup calls only `Project::create`, which journals
nothing (`grep -rn "NodeEvent::" crates/db/src/models/project/` → 0 matches) — and that invariant
is what a future author must preserve; (b) the sibling-alignment divergence the task ordered
declared: the 006 siblings (`task/queries.rs:341-347`, `hierarchy.rs:43-47`) use SELECT-first
DEFERRED transactions — the exact latent-517 shape — while this task now uses
IMMEDIATE+SELECT; sibling alignment is restored by new task 023, which converts both siblings to
the same IMMEDIATE shape; (c) the four factual corrections from panel B: the helper doc comment
cites node_outbox/task_breakdown/task::queries — not hierarchy; `TaskStatus` derives `sqlx::Type`,
not `FromRow`, and the honest macro-avoidance reason is not regenerating the offline cache; the
line refs in attempt 1's entry were stale on arrival; the test count was 319 (all -p db binaries)
/ 280 (--lib), not 273; (d) the `setup_test_pool` substitution for the dictated
`create_test_pool()` — functionally safe (it runs `sqlx::migrate!`, consistent with sync.rs's 18
prior uses) but undeclared until now.

## Allowed moves
ONLY: the transaction + probe restructure of `upsert_remote_task`'s body after the dirty-guard
(attempt 2: `begin_with("BEGIN IMMEDIATE")` + SELECT probe); the two append calls; moving the
existing statements onto the transaction; the `sync_event_tests` module including test 6. Do NOT
change the function's signature, the dirty-guard, the SQL text of the existing
`INSERT ... ON CONFLICT` statement, or any other function in the file. Do NOT touch the three dead
lifecycle writes. Nothing broadcasts here (the tailer publishes).

## STOP triggers
- The dirty-guard at `:271-279` or the upsert statement at `:283` differs from the shape cited
  here — STOP with the actual code.
- The probe's `query_as`/`query_scalar` typing of `status` as `TaskStatus` fails to compile (type
  mapping differs from expectation) — STOP with the compiler error; do not fall back to reading
  status as a `String`.
- `OutboxRepository` seeding for test 5 requires more than one repository call to set up — STOP
  and show what the existing outbox tests do; do not build bespoke outbox fixtures.
- Any need to alter a caller — the signature is unchanged, so there should be none.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): Rust crate — override the runner. Use
WAI_TYPECHECK_CMD="cargo fmt --all -- --check && cargo check --workspace --all-targets" with
WAI_TEST_CMD="cargo test -p db".

## Done when
`WAI_TYPECHECK_CMD="<typecheck>" WAI_TEST_CMD="<test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 022` exits 0
