---
id: "004"
phase: 1
title: "Add the event_journal model with append, cursor range-read, and compaction"
status: ready
depends_on: ["002","003"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/event_journal/mod.rs"
  - "crates/db/src/models/event_journal/queries.rs"
  - "crates/db/src/models/mod.rs"
siblings: ["crates/db/src/models/node_outbox.rs"]
irreversible: false
scope_test: "crates/db"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS1"]
---
## Failing test (write first)
**File:** `crates/db/src/models/event_journal/mod.rs` (colocated `#[cfg(test)] mod tests`),
using `db::test_utils::create_test_pool_with_migrations()` per CLAUDE.md — never hand-written
`CREATE TABLE`.

Tests (these ARE TS1):

1. `append_in_transaction_assigns_monotonic_seq` — append three events in one tx, commit; assert the
   returned seqs are strictly increasing.
2. `rollback_journals_nothing` — open a tx, append, then roll back; assert
   `SELECT COUNT(*) FROM event_journal` is 0. This is the D2 no-phantom-events guarantee.
3. `committed_seqs_are_strictly_increasing_across_rollback` — append (commit), append (rollback),
   append (commit); assert the two COMMITTED seqs are strictly increasing. Assert nothing about the
   value allocated inside the rolled-back transaction. SQLite **reuses** it — `sqlite_sequence` is
   itself transactional, so the rollback reverts the allocation (probed directly: committed 1,
   rolled-back allocation 2, next commit also 2). An earlier draft of this task asserted non-reuse
   and would have failed on correct SQLite behaviour. D9's consumer contract is unaffected: it says
   consumers must tolerate holes, which is the conservative direction whether or not this particular
   mechanism produces them.
4. `range_read_returns_exclusive_lower_inclusive_upper` — append 5, read `(2, 4]`; assert exactly
   seqs 3 and 4 come back, in ascending order.
5. `range_read_is_empty_above_high_water` — read `(5, 5]`; assert empty.
6. `compact_respects_retention_floor` — insert rows with a backdated `created_at`, compact with a
   retention window; assert old rows are gone and in-window rows remain.
7. `compact_never_crosses_min_trigger_cursor` — insert a `trigger_cursors` row with
   `last_processed_seq = N`, backdate ALL journal rows beyond retention, compact; assert every row
   with `seq >= N` survives. This is the D6 guarantee and the one most likely to be silently wrong.
8. `compact_retains_min_rows_floor` — with retention expired for everything, assert the newest
   `min_rows` rows survive.
9. `append_composes_with_a_caller_owned_transaction` — open a tx in the TEST, call `append(&mut *tx)`
   twice, commit in the test; assert both rows are present. This pins the executor-generic signature
   that task 006's delete path depends on: `Task::delete` is generic over `E: Executor` and its route
   at `crates/server/src/routes/tasks/handlers/core.rs:655-670` already owns the transaction, so an
   `append` that could only open its own would be unusable there.
10. `hard_cap_overrides_cursor_floor_and_flags_rebootstrap` — insert a `trigger_cursors` row with a
    LOW `last_processed_seq`, then insert more than `max_rows` journal rows above it; compact; assert
    (a) the row count drops to at most `max_rows`, (b) rows below the cursor floor WERE deleted, and
    (c) that cursor's `needs_rebootstrap` is now 1. This is the D6 hard-cap guarantee — without it a
    stopped hook pins the journal forever and the bounded-journal Constraint is unsatisfiable.


## Change
**File:** `crates/db/src/models/event_journal/mod.rs`
**Anchor:** new file — directory module, following the `crates/db/src/models/task_breakdown/`
two-file shape (mod.rs + queries.rs).
**After:** `mod queries;` plus the `EventJournalEntry` row struct (`seq: i64`, `event_type: String`,
`payload: String`, `created_at: DateTime<Utc>`) and the colocated `#[cfg(test)] mod tests`.

**File:** `crates/db/src/models/event_journal/queries.rs`
**Anchor:** new file
**Sibling to read FIRST:** `crates/db/src/models/node_outbox.rs` — the durable-ordered-log precedent.
List its structural choices (how it scopes reads by seq order, how it guards against duplicate seq,
its best-effort error posture where a failed enqueue is logged not propagated) and justify every
divergence in the ledger. Key expected divergence: this journal's append is NOT best-effort — it
shares the caller's transaction and its failure MUST propagate, because a silently dropped event
breaks the SC1/SC2 guarantee.

**Error type first.** These operations serialize and deserialize JSON, so they CANNOT return
`Result<_, sqlx::Error>` — `serde_json::Error` has no `From` conversion into `sqlx::Error`, and
`serde_json::to_string(event)?` would simply not compile. Define, in `mod.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum EventJournalError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("event payload serialization failed: {0}")]
    Serde(#[from] serde_json::Error),
}
```

Every operation below returns `Result<_, EventJournalError>`. This follows the `thiserror` + `#[from]`
rule in CLAUDE.md section 1.

**`append` is generic over the executor — this is load-bearing.** It must work both when a model
function opens its own transaction AND when one is handed a caller-owned transaction, because
`Task::delete` is already generic over `E: Executor` and its route owns an outer transaction spanning
child nullification (`crates/server/src/routes/tasks/handlers/core.rs:655-670`). An `append` that
opened its own transaction could never be used there — nested `begin()` on a generic executor is not
possible.

```rust
pub async fn append<'e, E>(executor: E, event: &NodeEvent) -> Result<i64, EventJournalError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
```

INSERT with `event.event_type()` and `serde_json::to_string(event)?`, returning the assigned `seq`.
`append` NEVER commits — committing is the caller's job in both composition modes.

Remaining operations (these read, so `&SqlitePool` is fine):

- `read_range(pool, after_seq: i64, through_seq: i64) -> Result<Vec<SequencedEvent>, EventJournalError>` —
  `WHERE seq > ? AND seq <= ? ORDER BY seq ASC`. Exclusive lower, inclusive upper: this is the
  `(cursor, mark]` window the spec's replay algorithm specifies.
- `high_water_mark(pool) -> Result<i64, EventJournalError>` —
  `SELECT COALESCE(MAX(seq), 0) FROM event_journal`.
- `compact(pool, retention_hours: i64, min_rows: i64, max_rows: i64) -> Result<u64, EventJournalError>` —
  two-stage, and the order matters:
  1. **Normal pass.** Delete rows that are BOTH older than the retention window AND outside the
     newest `min_rows`, AND strictly below the cursor floor
     `COALESCE((SELECT MIN(last_processed_seq) FROM trigger_cursors), <high_water>)`. When
     `trigger_cursors` is EMPTY there is no hook to protect, so there is no floor — the
     `COALESCE(..., high_water)` sentinel expresses exactly that, since deletion is strictly below it.
  2. **Hard cap.** If the journal still holds more than `max_rows` rows, delete the oldest rows down
     to `max_rows` **ignoring the cursor floor**, then set `needs_rebootstrap = 1` on every
     `trigger_cursors` row whose `last_processed_seq` is below the new minimum surviving `seq`. This
     is what makes the bounded-journal Constraint hold against a hook that has stopped advancing;
     the flag is how that hook learns it lost events rather than silently resuming mid-gap.

Deserialize `payload` back into `NodeEvent` via `serde_json::from_str` when building
`SequencedEvent`.

**File:** `crates/db/src/models/mod.rs`
**Anchor:** the module declaration list (currently L17-47; it has no `event_journal` entry)
**Change:** add `pub mod event_journal;` in alphabetical position. This is a required step, not a
contingency — a submodule is unreachable without it. (An earlier draft of this task left it as an
"only if cargo check demands it" aside while declaring `allowed_change: create`, which the file-set
gate rejects outright; task 009 declares the same edit correctly.)


## Allowed moves
ONLY the two new files plus the single `pub mod event_journal;` line in
`crates/db/src/models/mod.rs`. Do NOT wire emission, the broadcast sender, or the tailer — tasks
005/006/013. Do NOT spawn the compaction loop here; task 011 owns that and task 014 starts it.


## STOP triggers
- `create_test_pool_with_migrations()` does not exist in `crates/db/src/test_utils.rs` — use
  whatever the crate actually exposes and record it; do NOT hand-write `CREATE TABLE`.
- `thiserror` is not already a dependency of `crates/db` — check `crates/db/Cargo.toml` before
  writing `EventJournalError`; if absent, STOP rather than adding a dependency in a `create` task.
- Making `append` generic over `sqlx::Executor` will not compile against the `query!` macro form used
  by the sibling — STOP and record the exact error; the generic signature is load-bearing for task
  006's delete path and must not be quietly narrowed to `&SqlitePool`.
- The hard-cap pass cannot identify which cursors to flag without a second query — that is fine, do
  it in the same transaction as the cap deletion so a crash cannot delete rows without flagging.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db event_journal"

Record in the decisions-ledger: (a) the node_outbox sibling comparison and every justified
divergence, and (b) the observed rollback behaviour of SQLite AUTOINCREMENT. On the second: the
expected answer is that the value IS reused, because `sqlite_sequence` is transactional and the
rollback reverts the allocation — this was probed directly during decomposition. If you observe
non-reuse instead, say so in the ledger; the consumer contract holds either way, but the note should
match reality.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 004` exits 0
