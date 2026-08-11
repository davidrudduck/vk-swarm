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
siblings: ["crates/db/src/models/node_outbox.rs"]
irreversible: false
scope_test: "crates/db"
allowed_change: create
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
3. `seq_is_monotonic_across_rollback` — append (commit), append (rollback), append (commit); assert
   the third seq is strictly greater than the first and that NO seq is reused. Deliberately does NOT
   assert contiguity — D9 says a hole is legal. Document the observed behaviour in the ledger.
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

Three operations, all taking a `&mut Transaction` or executor rather than opening their own (the
CALLER — the state-writing model function — owns the transaction; see task 006):

- `append(tx, event: &NodeEvent) -> Result<i64, sqlx::Error>` — INSERT with
  `event.event_type()` and `serde_json::to_string(event)?`, returning the assigned `seq`.
- `read_range(pool, after_seq: i64, through_seq: i64) -> Result<Vec<SequencedEvent>, _>` —
  `WHERE seq > ? AND seq <= ? ORDER BY seq ASC`. Exclusive lower, inclusive upper: this is the
  `(cursor, mark]` window the spec's replay algorithm specifies.
- `high_water_mark(pool) -> Result<i64, _>` — `SELECT COALESCE(MAX(seq), 0) FROM event_journal`.
- `compact(pool, retention_hours: i64, min_rows: i64) -> Result<u64, _>` — delete rows that are
  BOTH older than the retention window AND outside the newest `min_rows`, AND strictly below
  `COALESCE((SELECT MIN(last_processed_seq) FROM trigger_cursors), <high_water>)`.

Deserialize `payload` back into `NodeEvent` via `serde_json::from_str` when building
`SequencedEvent`.


## Allowed moves
ONLY the two new files. Do NOT modify `crates/db/src/models/mod.rs` here — task 003
already added the models module list entry pattern; add `pub mod event_journal;` there ONLY if
`cargo check` demands it, and note it in the ledger. Do NOT wire emission or the broadcast sender —
tasks 005/006.


## STOP triggers
- `create_test_pool_with_migrations()` does not exist in `crates/db/src/test_utils.rs` — use
  whatever the crate actually exposes and record it; do NOT hand-write `CREATE TABLE`.
- The compaction predicate cannot be expressed without a subquery over `trigger_cursors` while that
  table is empty — decide the empty-table semantics (treat as high-water, i.e. no cursor floor) and
  record it in the ledger.
- `seq_is_monotonic_across_rollback` shows seq VALUES being reused after rollback — that would
  contradict D9's premise; STOP and escalate, the contract needs revisiting.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db event_journal"

Record in the decisions-ledger: (a) the node_outbox sibling comparison and every justified
divergence, and (b) the observed rollback behaviour of SQLite AUTOINCREMENT (does it leak a seq
value or not) — D9 assumes it may, and this is where we find out for real.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 004` exits 0
