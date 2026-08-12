---
id: "005"
phase: 2
title: "Add the EventBus with the broadcast channel and the subscribe_from replay-to-live contract"
status: ready
depends_on: ["004"]
parallel: false
conflicts_with: []
files:
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

Tests (these ARE TS2, together with task 013's tailer tests):

1. `subscribe_from_zero_replays_all_then_goes_live` — journal 3 events, `subscribe_from(0)`, assert
   seqs 1,2,3 arrive in order; then emit a 4th and assert it arrives live on the same stream.
2. `subscribe_from_cursor_skips_already_seen` — journal 5, `subscribe_from(3)`, assert only 4 and 5
   arrive.
3. `no_journaled_event_is_skipped_across_the_handoff` — the race the algorithm exists for: start
   `subscribe_from`, emit events CONCURRENTLY during the replay window, assert every journaled seq
   appears exactly once or more (duplicates tolerated) and none is missing. Assert on the SET of
   seqs, not the sequence, so a tolerated duplicate does not fail it.
4. `duplicates_are_tolerated_not_errors` — force a buffered live event whose seq was already
   replayed; assert the consumer-edge dedupe drops it and the stream continues.
5. `lagged_refills_from_journal_and_resumes_live` — use a deliberately tiny broadcast capacity,
   overrun it to force `RecvError::Lagged(n)`, and assert (a) the FULL set of journaled seqs written
   during the overrun is delivered, and (b) the stream then keeps delivering NEW live events. Part
   (b) is what catches a one-shot refill that recovers once and then dies.
6. `first_occurrences_are_ascending` — across all of the above, assert the seq of each event's FIRST
   occurrence is strictly ascending. Set-equality alone would pass an implementation that replays
   out of order.
7. `initial_read_error_surfaces_to_the_consumer` — point the bus at a closed/failed pool; assert
   `subscribe_from` yields an error rather than an empty stream. An empty stream is
   indistinguishable from "no events yet" and would silently strand a consumer.

Tests 5 and 6 are the ones that fail loudest if the `Lagged` branch is written as `continue` (which
skips journaled events) or as a single non-looping refill.


## Change
**Do NOT put the Sender in `crates/db`.** An earlier draft added a
`broadcast::Sender` field to `DBService`; the adversarial review proved that unbuildable. DB model
functions receive only `&SqlitePool`, and a pool has no back-reference to the `DBService` that owns
it, so a model function could never reach that sender to publish. A process-global was rejected too:
production constructs `DBService::bootstrap()` before the live service
(`crates/local-deployment/src/lib.rs:155-166`), so a single-assignment global captures the WRONG
sender, and `create_test_pool()` gives every test its own DB inside one shared process, so a global
would cross-publish between tests. Spec D8/D10 now place the Sender HERE, in `crates/services`, fed
by the journal tailer (task 013). `crates/db` needs no sender at all.

**File:** `crates/services/src/services/event_bus.rs`
**Anchor:** new file
**Cross-directory sibling to read FIRST:** `crates/services/src/services/events.rs` — the EXISTING
`EventService`. It is a different thing (SQLite-hook record patches over a msg-store) and must not be
merged with or renamed by this task, but read how it is constructed, held on the deployment, and
exposed, and follow the same shape. Justify any divergence in the ledger. Naming must stay
unambiguous: `EventService` = record patches, `EventBus` = the domain event log.

**After:** `EventBus` (Clone) holding the pool + a `broadcast::Sender<SequencedEvent>`, with:

- `pub fn sender(&self) -> broadcast::Sender<SequencedEvent>` — task 013's tailer publishes through
  this; nothing else may.
- `subscribe_from(&self, cursor: i64) -> Result<impl Stream<Item = Result<SequencedEvent, EventBusError>>, EventBusError>`

**The signature must be fallible.** Setup reads the journal (`high_water_mark`, `read_range`) and so
does `Lagged` recovery mid-stream; both can fail. An infallible `impl Stream<Item = SequencedEvent>`
leaves no way to report either, forcing a silent early end, a panic, or a swallowed error — and a
silently-ended stream is indistinguishable from an idle one. Define `EventBusError` wrapping
`EventJournalError`.

Implement it as an explicit LOOP, not a linear five-step procedure. The state machine, which is the
spec's algorithm made unambiguous:

```text
last = cursor
rx   = sender.subscribe()          // FIRST, before any journal read
loop {
    mark = high_water_mark(pool)?              // fresh mark every time we enter refill
    for ev in read_range(pool, last, mark)? {  // (last, mark], ascending
        yield ev; last = ev.seq
    }
    loop {                                     // live phase
        match rx.recv().await {
            Ok(ev)      => if ev.seq > last { yield ev; last = ev.seq }   // else: tolerated dup
            Err(Lagged(_)) => break,           // -> outer loop: refill from `last`
            Err(Closed) => return,
        }
    }
}
```

Four things this pins that prose did not:
1. Subscribe happens BEFORE the high-water read. Reversing them opens a window where an event
   committed in between is in neither the replay nor the live stream.
2. `Lagged(n)`'s `n` is a COUNT of skipped messages, not a seq. Never treat it as a cursor.
3. Refill captures a FRESH high-water mark each time — reusing the original mark would miss
   everything written since.
4. Recovery re-enters the live loop. A one-shot refill recovers once and then silently stops.

The invariant to hold in mind: every journal row with `seq > cursor` is delivered at least once, in
ascending order of first occurrence.

**File:** `crates/services/src/services/mod.rs`
**Change:** add `pub mod event_bus;` in alphabetical position.


## Allowed moves
ONLY the new event_bus.rs and the one module declaration line. Do NOT touch
`crates/db/src/lib.rs` — `DBService` gains no field in this plan. Do NOT touch
`crates/services/src/services/events.rs` or its `events/` directory. Do NOT write the tailer (task
013), instrument any emission site (phase 3), or add the SSE route (task 010).


## STOP triggers
- `subscribe_from` cannot be expressed as a `Stream` without an unstable feature — use
  `async_stream::try_stream!` if the workspace already has it, or return a boxed stream; record the
  concrete return type in the ledger. Do NOT drop the `Result` to make it compile.
- The workspace has no `Stream` combinator crate available at all — STOP; task 010's SSE route
  depends on this shape.
- You find yourself needing a sender inside `crates/db` to make a test pass — STOP. That is the exact
  design that was proven unbuildable; the tailer (013) is the answer.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services event_bus"

Record in the ledger: the `EventService` sibling comparison (why `EventBus` is a separate type rather
than an extension of it), the chosen broadcast capacity and its reasoning (a slow consumer that
exceeds it gets `Lagged` and refills from the journal, so this is a latency/memory knob, not a
correctness one), and the concrete `subscribe_from` return type.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 005` exits 0
