---
id: "005"
phase: 2
title: "Add the broadcast sender to DBService and the EventBus wrapper with subscribe_from"
status: ready
depends_on: ["004"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/lib.rs"
  - "crates/services/src/services/event_bus.rs"
  - "crates/services/src/services/mod.rs"
siblings: ["crates/services/src/services/events.rs"]
irreversible: false
scope_test: "crates/services"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS2"]
---
## Failing test (write first)
**File:** `crates/services/src/services/event_bus.rs` (colocated `#[cfg(test)] mod tests`),
using `db::test_utils::create_test_pool_with_migrations()`.

Tests (these ARE TS2):

1. `broadcast_only_after_commit` — subscribe, append+broadcast inside a transaction that is then
   ROLLED BACK; assert the subscriber receives nothing. Then commit a second one and assert it
   arrives. Pins D2.
2. `subscribe_from_zero_replays_all_then_goes_live` — journal 3 events, `subscribe_from(0)`, assert
   seqs 1,2,3 arrive in order; then emit a 4th and assert it arrives live on the same stream.
3. `subscribe_from_cursor_skips_already_seen` — journal 5, `subscribe_from(3)`, assert only 4 and 5
   arrive.
4. `no_journaled_event_is_skipped_across_the_handoff` — the race the algorithm exists for: start
   `subscribe_from`, emit events CONCURRENTLY during the replay window, assert every journaled seq
   appears exactly once or more (duplicates tolerated) and none is missing. Assert on the SET of
   seqs, not the sequence, so a tolerated duplicate does not fail it.
5. `duplicates_are_tolerated_not_errors` — force a buffered live event whose seq was already
   replayed; assert the consumer-edge dedupe drops it and the stream continues.
6. `lagged_refills_from_journal` — use a deliberately tiny broadcast capacity, overrun it to force
   `RecvError::Lagged(n)`, and assert the stream recovers by re-reading the journal from the last
   delivered seq with no journaled event skipped.

Test 6 is the one that fails loudest if the Lagged branch is written as `continue` instead of a
journal refill.


## Change
**File:** `crates/db/src/lib.rs`
**Anchor:** `pub struct DBService` at L307 — currently `{ pub pool: Pool<Sqlite>, pub metrics: DbMetrics }`,
`#[derive(Clone)]`.
**Change:** add a third field holding
`tokio::sync::broadcast::Sender<db::models::event::SequencedEvent>`, constructed with
`broadcast::channel(<capacity>).0` in BOTH constructors (`bootstrap()` at L315 and `new()` at L340).
`Sender` is `Clone`, so `#[derive(Clone)]` still holds. Expose a getter returning `&Sender` and a
`subscribe()` returning a `Receiver`.

Rationale (spec D8): `crates/db` cannot depend on `crates/services`, and the post-commit publish must
happen where the transaction commits — which is the db model function. So the Sender lives here.
`crates/db` already depends on tokio (`crates/db/Cargo.toml:20`).

Pick the capacity as a named `const` with a comment — a slow consumer that exceeds it gets
`Lagged(n)` and refills from the journal, so this is a latency/memory knob, not a correctness one.

**File:** `crates/services/src/services/event_bus.rs`
**Anchor:** new file
**Cross-directory sibling to read FIRST:** `crates/services/src/services/events.rs` — the EXISTING
`EventService`. It is a different thing (SQLite-hook record patches over a msg-store) and must not be
merged with or renamed by this task, but read how it is constructed, held on the deployment, and
exposed, and follow the same shape. Justify any divergence in the ledger. Naming must stay
unambiguous: `EventService` = record patches, `EventBus` = the domain event log.

**After:** `EventBus` (Clone) wrapping the pool + the Sender, with:
- `subscribe_from(&self, cursor: i64) -> impl Stream<Item = SequencedEvent>` implementing the spec's
  EXACT five-step algorithm, in this order:
  1. subscribe to the live broadcast channel FIRST (before reading the journal),
  2. capture `high_water_mark(pool)`,
  3. replay `read_range(cursor, mark]` in seq order,
  4. drain the live receiver, DISCARDING any buffered event with `seq <= last_replayed`,
  5. on `RecvError::Lagged(n)`, re-enter journal refill from the last DELIVERED seq before resuming
     live — never `continue`, which would skip journaled events.

Step order is load-bearing: subscribing after reading the high-water mark opens a window where an
event committed in between is in neither the replay nor the live stream.

**File:** `crates/services/src/services/mod.rs`
**Change:** add `pub mod event_bus;` in alphabetical position.


## Allowed moves
ONLY the DBService field + constructors + accessors, the new event_bus.rs, and the
module declaration. Do NOT touch `crates/services/src/services/events.rs` or its `events/`
directory. Do NOT instrument any emission site — that is phase 3. Do NOT add the SSE route.


## STOP triggers
- `DBService` has constructors beyond `bootstrap()` and `new()` that also build a pool — every one
  must get the Sender or the bus is silently dead in that path. Enumerate them before editing.
- Adding the field breaks `#[derive(Clone)]` — it should not; `broadcast::Sender` is Clone. If it
  does, STOP.
- `subscribe_from` cannot be expressed as a `Stream` without an unstable feature — record the
  concrete return type chosen in the ledger.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services event_bus"

Record in the ledger: the `EventService` sibling comparison (why `EventBus` is a separate type rather
than an extension of it), the chosen broadcast capacity and its reasoning, and the concrete
`subscribe_from` return type.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 005` exits 0
