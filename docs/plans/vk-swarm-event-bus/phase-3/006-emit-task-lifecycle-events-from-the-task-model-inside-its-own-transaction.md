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
  - "crates/db/src/models/activity_dismissal.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: []
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
6. `delete_journals_inside_the_callers_transaction` — open a transaction in the TEST, call
   `Task::nullify_children_by_parent_id(&mut *tx, ..)` then `Task::delete(&mut *tx, id)`, then ROLL
   BACK; assert the task still exists AND `event_journal` has no `task_deleted` row. Then repeat with
   a commit and assert both landed. This pins that delete appends on the caller's executor rather
   than committing its own transaction — the behaviour the real route at
   `crates/server/src/routes/tasks/handlers/core.rs:655-670` depends on.
7. `update_status_with_existing_dismissal_succeeds` — create a task WITH an activity dismissal, then
   `update_status`; assert it completes without deadlock, the dismissal is cleared, and exactly one
   `task_status_changed` row exists. Without this the dismissal path is never exercised inside a
   transaction.

There is deliberately NO broadcast assertion here. Model functions append; the tailer publishes
(task 013). A test asserting broadcast at this layer would be testing the tailer through the wrong
seam.

## Change

**Query form for any NEW SQL you write (amended 2026-08-12).** Use the runtime sqlx API —
`sqlx::query(...)`, `sqlx::query_as::<_, Row>(...)`, `sqlx::query_scalar::<_, T>(...)` plus
`.bind()`. Do NOT write a NEW `sqlx::query!` / `query_as!` / `query_scalar!` macro call (re-using an
EXISTING macro query verbatim is fine — it is already cached). Reason, established by probe and
recorded in full in task 004's Change section: this crate's `.sqlx` offline query cache is tracked,
compile-time verification is active, and a new macro query would require `cargo sqlx prepare` whose
`query-<hash>.json` output cannot be declared in `files:` — the committer would leave it unstaged, so
the build would work here and nowhere else. STOP if you find yourself needing `cargo sqlx prepare`.
The spec's D2 "Emission ownership" rule has TWO shapes, and picking the wrong one per site
is the failure mode this task exists to prevent:

- **Pool-taking sites** (`Task::create`, `Task::update`, `Task::update_status`): the model function
  opens its own transaction, performs its EXISTING discrete statement inside it, appends the journal
  row, and commits.
- **Executor-taking sites** (`Task::delete`): the model function appends on the executor it was
  GIVEN and does NOT commit — the caller already owns the transaction and commits it.

**No site broadcasts.** Model functions append only; the tailer (task 013) publishes what it reads
back from the journal. That is what makes "never broadcast before commit" structural rather than a
rule an implementer has to remember.

Caller signatures stay unchanged in both shapes — which is exactly why the node_outbox precedent's
objection (`crates/db/src/models/task/queries.rs:337`, "threading a shared txn through all
`Task::create` callers is OUT of scope") does not apply.

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
`NodeEvent::TaskCreated { .. }` via `event_journal::append(&mut *tx, &event)`, `tx.commit().await?`,
then `Self::enqueue_task_upsert_op(pool, &task).await;` (which stays OUTSIDE the transaction — it is
best-effort by design and must not be able to roll back the task write), then `Ok(task)`.

**Anchor:** `Task::update`, the identical `.fetch_one(pool)` / `enqueue_task_upsert_op` pair at
L327-330.
**After:** same shape. Emit `task_status_changed` ONLY when the status actually differs — read the
prior row inside the transaction to compare (see failing test 4).

**Anchor:** `Task::delete` at L369-376 — generic over `E: Executor`, NOT `&SqlitePool`.
**After:** append onto the executor it was GIVEN; do not open a transaction and do not commit.

This is the one site where "the model opens its own transaction" cannot apply, and it is not a corner
case — it is the primary user-delete path. `crates/server/src/routes/tasks/handlers/core.rs:655-670`
already opens a transaction, calls `Task::nullify_children_by_parent_id(&mut *tx, …)`, then
`Task::delete(&mut *tx, task.id)`, then commits, precisely so child nullification and deletion are
atomic. A nested `begin()` on a generic consumed executor is not expressible, and an inner commit
would break that atomicity.

Because `event_journal::append` is generic over `E: Executor` (task 004), the fix is simply to append
on the same executor:

```rust
pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<u64, …>
where E: Executor<'e, Database = Sqlite>
{
    // load identity for the payload, DELETE, then append TaskDeleted on the SAME executor
}
```

The caller's commit then makes the deletion and its journal row atomic together — which is exactly
what D2 requires. No route changes, no new `delete_with_event` entry point, no caller migration.

Note the ordering constraint: the event payload needs `project_id`, so read it BEFORE the DELETE, on
the same executor.

**File:** `crates/db/src/models/task/hierarchy.rs`
**Anchor:** `Task::update_status` at L13-29 — the status-change path used by
`ContainerService::start_execution`.
**After:** same transaction-owning shape, BUT read L18-29 first: after updating the task it calls an
activity-dismissal helper that takes `&SqlitePool` only
(`crates/db/src/models/activity_dismissal.rs:49-53`). This is a genuine hazard — calling a
pool-taking helper while your own transaction holds SQLite's single writer lock can self-block on a
second connection, and moving it after the commit means it can fail after the event is already
journaled.

Resolution: generalize that helper to accept an executor and call it INSIDE the transaction, so the
order is: update status → clear dismissal → append event → commit. Add
`crates/db/src/models/activity_dismissal.rs` to this task's `files:`. Add a test that exercises
`update_status` on a task WITH an existing dismissal — without it, this path is never covered.

## Allowed moves
ONLY the transaction wrapping and journal append at the four named functions, plus
generalizing the activity-dismissal helper's executor parameter. **Nothing here broadcasts** — model
functions append only; publication is the tailer's job (task 013). Do NOT change any function's
parameters or return type apart from the dismissal helper's executor generalization. Do NOT move
`enqueue_task_upsert_op` inside the transaction — it is deliberately best-effort and outside. Do NOT
touch other files in `crates/db/src/models/task/` (archive.rs, sync.rs, cleanup.rs).

## STOP triggers

**Two of these are PRE-RESOLVED by the orchestrator (2026-08-12) — do not spend a STOP on them:**
- *Raw status writes bypassing the four functions:* enumerated with
  `git grep -n "SET status" -- 'crates/**/*.rs'`. The ONLY write to `tasks.status` in Rust source is
  `crates/db/src/models/task/hierarchy.rs:19`, which IS `update_status` itself. `Task::update`'s
  status write is inside its own `UPDATE ... SET title, description, status, parent_task_id`. There
  is no bypass path for STATUS. ~~so SC1 coverage is complete with the four named functions.~~
  **COMPLETENESS CLAIM STRUCK 2026-08-15.** The status half above is still true and was re-verified
  that day. The *completeness* half is FALSE for CREATION: `task_breakdown::accept_proposal`
  (`crates/db/src/models/task_breakdown/queries.rs:406`, routed at `breakdown.rs:273`) and the two
  hive-sync paths (`task/sync.rs:32`, `:283`) all `INSERT INTO tasks` without going through
  `Task::create`. `task_breakdown` merged in PR #475 on 2026-08-11, concurrent with this decompose.
  **Task 020 covers the breakdown site; the sync paths are a separate open decision.** Nothing about
  this changes YOUR scope — instrument exactly the four functions named below and no others — but do
  not repeat the completeness claim in the ledger, and do not treat the four functions as proof that
  SC1 is fully covered by this task.
  `crates/db/src/models/task/archive.rs:15` writes `archived_at`, NOT `status` — a separate lifecycle
  concern this plan does not journal, and archive.rs stays out of `files:` as stated above.
- *Dismissal-helper callers:* enumerated with `git grep -n "clear_for_task\|undismiss"`.
  `clear_for_task` has exactly ONE caller (`hierarchy.rs:27`), and `undismiss` has one caller outside
  the model (`crates/server/src/routes/dashboard.rs:62`, passing `&deployment.db().pool`). Generalizing
  to `E: Executor` keeps that caller compiling unchanged, because `&SqlitePool` implements `Executor`.
  Generalize `clear_for_task`; you may leave `undismiss` pool-taking if that is simpler.

- You are about to give `Task::delete` its own transaction, or add a `delete_with_event(pool, id)`
  entry point — STOP and re-read the Change section. The delete route owns the transaction; appending
  on the passed executor is the whole point of the generic signature.
- `event_journal::append` turns out NOT to be generic over `Executor` — STOP; task 004 owes that
  signature and the delete path cannot work without it.
- Another code path writes task status directly with raw SQL, bypassing these four functions
  (`git grep -n "UPDATE tasks"`) — every such path is a missed event; enumerate them and STOP.
- Generalizing the dismissal helper breaks an unlisted caller — enumerate with
  `git grep -n "clear_for_task\|undismiss"` and STOP rather than editing outside `files:`.
- Wrapping in a transaction causes `database is locked` in tests — that indicates an enclosing
  transaction or a pool-taking helper called from inside the transaction; STOP rather than adding
  retries.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db task"

Live SC1 check (record output in the ledger): on a running node, create a task, move it, delete it,
then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'task_%' order by seq"`
shows exactly three rows in that order with strictly increasing seq.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 006` exits 0
