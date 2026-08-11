---
id: "006"
phase: 3
title: "Emit task lifecycle events from the task model inside its own transaction"
status: ready
depends_on: ["005"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/task/queries.rs"
  - "crates/db/src/models/task/hierarchy.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: ["SC1"]
covers_tests: []
---
## Failing test (write first)
**File:** `crates/db/src/models/task/queries.rs` (extend the colocated tests), using
`db::test_utils::create_test_pool_with_migrations()`.

1. `create_emits_task_created` — `Task::create`, then assert exactly ONE `event_journal` row with
   `event_type = 'task_created'` whose payload carries the new task id and project id.
2. `update_status_emits_task_status_changed_with_both_statuses` — move a task todo → inprogress;
   assert one `task_status_changed` row carrying BOTH old and new status. Reading the old status
   must happen inside the same transaction as the update, or the old value can be lost to a
   concurrent write.
3. `delete_emits_task_deleted` — assert one `task_deleted` row.
4. `update_without_status_change_emits_no_status_event` — `Task::update` changing only the title
   must NOT produce a `task_status_changed` row. This is the "exactly one event per state change"
   half that is easy to get wrong by emitting unconditionally.
5. `failed_write_journals_nothing` — force the state write to fail (e.g. violate a FK by using an
   absent project_id); assert `event_journal` is empty. Proves the shared transaction.
6. `event_is_broadcast_after_commit` — subscribe to the bus, run `Task::create`, assert the
   subscriber receives the event and that its seq matches the journaled row.


## Change
For EACH function below the shape is identical, and it is the spec's D2 "Emission ownership"
rule: **the model function opens its own transaction, performs its EXISTING discrete statement inside
it, appends the journal row, commits, then broadcasts.** Caller signatures stay `pool: &SqlitePool` —
nothing is threaded through callers, which is exactly why the node_outbox precedent's objection
(`crates/db/src/models/task/queries.rs:337`, "threading a shared txn through all `Task::create`
callers is OUT of scope") does not apply.

**File:** `crates/db/src/models/task/queries.rs`
**Anchor:** `Task::create`, the `.fetch_one(pool)` at L290 followed by
`Self::enqueue_task_upsert_op(pool, &task).await;` at L292.
**Before:**
```rust
        .fetch_one(pool)
        .await?;
        Self::enqueue_task_upsert_op(pool, &task).await;
        Ok(task)
```
**After:** begin a transaction, run the same `query_as!` against `&mut *tx`, append
`NodeEvent::TaskCreated { .. }` via `event_journal::append(&mut tx, &event)`, `tx.commit().await?`,
then publish `SequencedEvent { seq, event }` on the DBService sender, then
`Self::enqueue_task_upsert_op(pool, &task).await;` (which stays OUTSIDE the transaction — it is
best-effort by design and must not be able to roll back the task write), then `Ok(task)`.

**Anchor:** `Task::update`, the identical `.fetch_one(pool)` / `enqueue_task_upsert_op` pair at
L327-330.
**After:** same shape. Emit `task_status_changed` ONLY when the status actually differs — read the
prior row inside the transaction to compare (see failing test 4).

**Anchor:** `Task::delete` at L369 — note it is generic over `E: Executor`, not `&SqlitePool`.
**After:** it must gain a transaction internally too. If the generic executor signature makes that
impossible without touching callers, add a `delete_with_event(pool, id)` alongside it and route the
callers that represent real user deletions; record the decision and the caller list in the ledger.

**File:** `crates/db/src/models/task/hierarchy.rs`
**Anchor:** `Task::update_status` at L13 — this is the status-change path used by
`ContainerService::start_execution` (`crates/services/src/services/container.rs:1503`).
**After:** same shape; emit `task_status_changed` with old and new status.


## Allowed moves
ONLY the transaction wrapping, the journal append, and the post-commit broadcast at
the four named functions. Do NOT change any function's parameters or return type. Do NOT move
`enqueue_task_upsert_op` inside the transaction — it is deliberately best-effort and outside. Do NOT
touch other files in `crates/db/src/models/task/` (archive.rs, sync.rs, cleanup.rs).


## STOP triggers
- `Task::delete`'s generic executor cannot be given a transaction without changing callers — STOP and
  record before choosing the `delete_with_event` fallback.
- Another code path writes task status directly with raw SQL, bypassing these four functions
  (`git grep -n "UPDATE tasks"`) — every such path is a missed event; enumerate them and STOP.
- Wrapping in a transaction causes `database is locked` in tests — that indicates an enclosing
  transaction or a held connection; STOP rather than adding retries.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db task"

Live SC1 check (record output in the ledger): on a running node, create a task, move it, delete it,
then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'task_%' order by seq"`
shows exactly three rows in that order with strictly increasing seq.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 006` exits 0
