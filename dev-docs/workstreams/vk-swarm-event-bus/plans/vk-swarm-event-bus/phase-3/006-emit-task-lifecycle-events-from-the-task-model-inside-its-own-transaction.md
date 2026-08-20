---
id: "006"
phase: 3
title: "Emit task lifecycle events from the task model inside its own transaction"
status: passed
depends_on: ["005"]
parallel: false
conflicts_with: ["023"]
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

---

## REQUIRED — after orchestrator review, before gating

`Task::delete` has **two** production call sites, not one. This task file and `plan.md` both assert
one. That was my error and it is corrected here:

```text
core.rs:663:   Task::delete(&mut *tx, task.id).await?          <- transaction
remote.rs:254: Task::delete(&deployment.db().pool, task.id)    <- POOL, no transaction
```

`Acquire::acquire` on `&mut Transaction` is a pure passthrough (sqlx-core-0.8.6
`transaction.rs:250`), so the transaction path is correct as implemented. On a pool it yields a
`PoolConnection` and the three statements become three auto-commits — a failed append after a
successful DELETE deletes the task and journals nothing.

**REQUIRED: use `Acquire::begin` instead of `Acquire::acquire` in `Task::delete`**, and commit the
resulting transaction before returning. On a pool this opens a real transaction; on `&mut Transaction`
it opens `SAVEPOINT _sqlx_savepoint_{depth}` (`transaction.rs:281`) nested in the caller's
transaction. Both paths become atomic and no caller changes.

**The earlier STOP trigger forbidding delete "its own transaction" is superseded for this specific
purpose.** It was written assuming a single tx-owning caller, and its concern was holding the SQLite
writer lock across unrelated work; a savepoint inside an already-open transaction does not extend a
lock the outer transaction already holds. Do NOT add a `delete_with_event` entry point — that part of
the trigger stands.

**REQUIRED test:** prove the pool path is atomic. Make the append fail after the DELETE has
succeeded (corrupt the payload, or point the append at a renamed `event_journal` table — the tailer
tests in `crates/services` use `ALTER TABLE ... RENAME TO ...` for exactly this), call
`Task::delete(&pool, id)`, and assert **the task still exists**. Prove it bites: with `.acquire()`
restored, this test must FAIL. Paste both runs.

Note `chmod` and closing the pool do NOT inject a usable fault here — see the sqlx fault-injection
notes; a table rename or payload corruption is what works.

Keep the `impl Future` return shape and its `#[allow(clippy::manual_async_fn)]`; the HRTB limitation
that forced it is unrelated to this change and still applies.

---

## REQUIRED — attempt 2, after panels 15A and 15B

Two panels reviewed attempt 1 with disjoint remits. **Neither found a blocking defect and both
concluded the production code is correct.** 15A's framing: *"I proved the code right and the tests
thin."* 15B affirmatively proved the savepoint-ordering claim by experiment.

**Do not redo the implementation.** The emission logic, the `Acquire` bound, `.begin()`, and every
payload are correct and stay. Eight remediations follow: seven are tests or comments, one is a
two-line signature simplification and one a single-statement SQL change.

**Most of the test source already exists and is verified.** Panel 15A's six probes are in
`/tmp/claude-1000/-data-Code-vk-swarm/7ada6c82-d888-446d-9d5c-48560bedfbbb/scratchpad/panel15a-probes.rs.txt`
(green on clean code, red on each mutation). Panel 15B's two savepoint tests plus the A2 guard and
the `assert_send` net are in `.wai-scratch/panel15b-savepoint-tests.rs` (green on shipped `.begin()`,
red on `.acquire()`). **Use them. Do not re-derive them** — the whole of finding 5 below is that
these tests are easy to write wrong, and both panels wrote them wrong at least once before getting
them right. Adapt only what does not compile.

### 1. No-op `update_status` must emit nothing (15A-1)

Deleting `&& old_status != status` from `hierarchy.rs:62-64` survives the entire crate suite
(`ok. 236 passed; 0 failed`). Add the negative-path test. The guard is load-bearing: seven production
writers of `Done`/`InReview` (`git_ops.rs:99`, `github.rs:279`, `pr_monitor.rs:186`, `:259`,
`container.rs:296`, `:597`, `:1594`) call `update_status` without checking current status, while
`approvals.rs:465` does gate on it — the asymmetry is what the guard absorbs.

**Prove it bites:** delete the guard, the new test must FAIL. Paste both runs.

### 2. Append-failure atomicity for the three pool-taking sites (15A-2)

`Task::create`, `Task::update`, `Task::update_status` have no test forcing the APPEND to fail.
`failed_write_journals_nothing` runs the opposite direction, which is why the axis reads covered.
Swallowing the append error in `Task::create` — a committed task with no journal row, the exact SC1
violation — survives every shipped test.

Add 15A's three probes. Include its sub-gap: pin that the **dismissal clear** rides the transaction
too, which is the single reason `clear_for_task` was generalised.

### 3. Pin `task_id` in `Task::update`'s event (15A-3)

`update_with_status_change_emits_task_status_changed` destructures `{ old_status, new_status, .. }`,
so `task_id: Uuid::nil()` survives. Name `task_id` and assert it.

### 4. Correct a false rationale in a test comment (15A-5)

`queries.rs:1139-1140` justifies the table repair by "the process-wide template database other tests
copy from". These tests use `create_test_pool_with_migrations`, which builds a fresh `TempDir` per
call (`test_utils.rs:107-131`) and never touches the template `create_test_pool` uses. The repair is
good hygiene; the reason is wrong.

### 5. The savepoint test is VACUOUS — pair it, do not delete it (15B-1)

`delete_via_savepoint_rolls_back_cleanly_on_append_failure` **passes against the exact `.acquire()`
defect it was added to disprove**, because its final act rolls the outer transaction back, making
"the task still exists" true either way.

**This was my specification error, not yours.** I required that assertion verbatim.

**KEEP the existing test** — it rules out a poisoned connection, which is a real property. **ADD**
`delete_savepoint_failure_is_undone_even_if_the_caller_commits` and
`failed_savepoint_leaves_the_outer_transaction_usable` from `.wai-scratch/panel15b-savepoint-tests.rs`.
The discriminator is COMMITTING the outer transaction rather than rolling it back. Also apply the A2
error-identity guard to the existing pool test.

**Prove it bites:** with `.acquire()` restored, the two new tests must FAIL while the existing
savepoint test still passes. That single run is the finding.

### 6. Collapse the HRTB workaround (15B-2)

The `impl Future` + split `'a`/`'c` + `#[allow(clippy::manual_async_fn)]` shape was required by the
`.acquire()` body, not the `.begin()` one. Nobody re-tested after the switch, and the doc comment now
tells the next reader that simplifying will break the build — false, and load-bearing guidance.

Collapse to `pub async fn delete<'c, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error> where
E: Acquire<'c, Database = Sqlite> + Send`, drop the `#[allow]`, and rewrite the comment to record
what actually happened: `.acquire()` forced the HRTB obligation because
`Acquire::Connection = &'c mut SqliteConnection` carries the bound's lifetime through the reborrow;
`.begin()` returns an owned `Transaction<'c, _>` and dissolves it.

**Add the `assert_send` compile test** from the scratch file. `async fn` infers `Send` where
`impl Future + Send` asserts it, so without it a future caller breaking Send-ness fails at that
caller with an opaque axum `Handler` error instead of here.

**If `cargo check --workspace --all-targets` does not pass after collapsing, STOP and report** —
15B proved it clean, but its proof is not a substitute for yours.

### 7. Remove the read-then-upgrade (15B-3)

`.begin()` on the POOL path makes a deferred transaction that reads then upgrades to a write, and
SQLite does not invoke the busy handler for that upgrade: 6 failures in 40 under contention versus 0
with `.acquire()`. **Atomicity held in every run** (`journaled == ok`) — an error-rate cost, not a
torn write, and `.acquire()` traded a retryable error for a torn write, so `.begin()` stays.

Replace the separate `SELECT project_id` + `DELETE` with a single
`DELETE FROM tasks WHERE id = $1 RETURNING project_id`. Write-first, no upgrade, one round trip
fewer. Runtime API, not a macro. If `RETURNING` cannot give you what the event needs, STOP and say
so rather than reverting to two statements silently.

### 8. Three call sites, not two (15A-4, 15B-4)

My amendment and three ledger sections say two. Correct the count wherever it appears in the task
file and ledger. **No code change** — `remote.rs:266` is pool-shaped exactly like `:254` and already
covered.

## Verification for attempt 2

`cargo test -p db`, `cargo fmt --all -- --check`, `cargo clippy -p db --all-targets --all-features
-- -D warnings`, `cargo check --workspace --all-targets` — all exit 0. Plus the two bite proofs
(items 1 and 5) verbatim.
