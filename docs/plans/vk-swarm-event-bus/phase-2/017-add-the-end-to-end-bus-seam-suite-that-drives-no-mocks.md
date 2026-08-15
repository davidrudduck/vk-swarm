---
id: "017"
phase: 2
title: "Add the end-to-end bus seam suite that hand-drives nothing"
status: passed
depends_on: ["013","016"]
parallel: false
conflicts_with: []
files:
  - "crates/services/tests/event_bus_end_to_end.rs"
siblings:
  - "crates/services/tests/electric_task_sync.rs"
  - "crates/services/tests/filesystem_repo_discovery.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: create
covers_criteria: []
covers_tests: []
---

## Why this task exists

Panel 6 recorded an observation it could not turn into a mutation finding, because both paths share
one `sender` and no mutation isolates it:

> no single test covers the real end-to-end path `commit → tailer → broadcast → subscribe_from`.
> Every `subscribe_from` test hand-drives `sender` with fabricated `SequencedEvent`s; every tailer
> test uses a raw `tx.subscribe()` receiver.

After 013 and 016 the module has 268 tests proving the **parts** and none proving the **whole**. That
is the exact shape of defect the run-level reachability gate exists to catch — its requirement (b) is
"at least one test drives the real entry point / integration seam, not a mock past it", and a task
whose only test calls the changed unit directly FAILS that gate. **This run cannot be declared done
without this suite**, so it is a task rather than a note.

It sits in phase 2 because it needs only the bus itself. The HTTP-level seam (a real `GET
/api/events` request observing an event caused by a real task mutation) is a separate obligation
recorded against task 014.

## Failing test (write first)

**File:** `crates/services/tests/event_bus_end_to_end.rs` — a new integration suite.

**The binding constraint for every test here: never construct a `SequencedEvent` by hand and never
call `sender().send(..)`.** Events must enter the system ONLY by being appended to the journal
through the real model API inside a real transaction, and must be observed ONLY through
`EventBus::subscribe_from`. Any test that shortcuts either end defeats the purpose of the suite and
should be deleted rather than weakened.

1. `a_committed_row_reaches_a_live_subscriber` — construct `EventBus::new`, take
   `subscribe_from(0)`, append one event inside a transaction, commit, and assert the subscriber
   yields it with its absolute seq and its full body intact.
2. `a_subscriber_that_joins_late_replays_from_its_cursor_then_goes_live` — commit three events, THEN
   `subscribe_from(0)`, assert all three replay in seq order, then commit a fourth and assert it
   arrives on the same stream with no gap and no duplicate across the handoff.
3. `a_rolled_back_transaction_reaches_no_subscriber` — append inside a transaction, roll back, commit
   a different event, and assert the subscriber sees only the second. Journal-first means an
   uncommitted event must be invisible end to end, not merely absent from the journal.
4. `a_new_bus_on_the_same_pool_resumes_without_replaying_history` — drop the first bus, build a
   second on the same pool, and assert a subscriber started at the high-water mark sees only events
   committed after the restart. This is the durability property task 012 must later observe live.
5. `every_event_variant_survives_the_full_round_trip` — one of each `NodeEvent` variant through the
   whole path, comparing full serialized bodies. The unit suite proves the tailer republishes every
   variant; this proves nothing between the journal and the subscriber narrows them.

## Change

- **File:** `crates/services/tests/event_bus_end_to_end.rs`
- **Anchor:** new file.
- **After:** the five tests above, using `db::test_utils::create_test_pool_with_migrations()` for the
  pool. Never hand-write `CREATE TABLE` (CLAUDE.md — schema drift produces false greens).
- Read the sibling `crates/services/tests/electric_task_sync.rs` first and follow its structure for
  pool setup, async test attributes, and teardown. Record any deliberate divergence in the ledger.
- Waiting must be deadline-based with a generous bound (the ledger records repeated flakiness in this
  module from fixed sleeps). Never assert on elapsed time.

## Allowed moves

- ONLY the creation of this one file. No production code changes: if a test cannot be written without
  changing the bus's public surface, that is a STOP, not a licence to edit `mod.rs`.
- No `#[ignore]` on any test in this file.

## STOP triggers

- Any of the five tests cannot be written without hand-driving `sender` or fabricating a
  `SequencedEvent` — report which and why rather than writing a weaker test that looks equivalent.
- `EventBus`, `subscribe_from`, or the journal append path is not public enough to drive from an
  integration test. Widening visibility is a production change and outside this task's file set.
- A test fails in a way that indicates a REAL defect in 013/016 rather than a defect in the test.
  That is the suite doing its job — stop and report it; do not adjust the assertion to pass.
- `crates/services/tests/normalize_sync_test.rs` fails: KNOWN pre-existing load-sensitive flake
  (tracked in `dev-docs/workstreams/normalize-fast-execution-lost-logs-flake/`). Re-run, confirm no
  OTHER test failed, do not touch that file.

## Done when
`WAI_TYPECHECK_CMD="cargo check --workspace" WAI_TEST_CMD='cargo test -p "$(basename {scope})"' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 017` exits 0
