---
id: "016"
phase: 2
title: "Make the tailer give-up defect unrepresentable, and the tailer observable"
status: ready
depends_on: ["013"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/event_bus/tailer.rs"
  - "crates/services/src/services/event_bus/mod.rs"
siblings: []
irreversible: false
scope_test: "crates/services"
allowed_change: edit
covers_criteria: []
covers_tests: []
---

## Why this task exists

Six adversarial panels attacked task 013. Two of their findings — panel 5's "retry duration" and
panel 6's "idle persistence" — are the SAME defect class: *the poll loop terminates early under
condition C*. Error-C and idle-C were found one round apart. Remaining members of the family
(lagged, pool-exhausted, panic-in-body) have not been enumerated and there is no reason to think
the list is finished.

Every remedy so far has been "extend a wall-clock window past the mutant's budget". That approach
cannot close the class: the ledger already carries a DECLARED RESIDUAL that a give-up budget of 100
still passes an 8000ms window, and each new member costs another multi-second sleep. The module's
suite went from ~6s to ~11-13s buying two members of an infinite family.

The reason the suite cannot state "the tailer never gives up" is that the production code does not
expose it: an opaque `tokio::spawn` whose liveness is only inferable from timing side-effects.
**This task changes the shape of the code so the defect cannot be written, instead of continuing to
detect instances of it.**

It also closes a product gap every blast-radius analysis in the ledger has named and none has
actioned: if the tailer dies, nothing in production notices. The channel goes quiet, `subscribe_from`
parks forever, and every health surface still reads green.

## Failing test (write first)

**File:** `crates/services/src/services/event_bus/tailer.rs`, test module.

### 1. `a_poll_step_can_never_terminate_the_loop` — the structural guarantee, zero sleeps

Drive `poll_once` DIRECTLY, synchronously, with no tailer task spawned and no `sleep` anywhere:

- against an EMPTY journal, called 50 times in a row — every call returns `PollOutcome::Idle`
- against an unreadable journal (table renamed away), called 50 times — every call returns
  `PollOutcome::Failed`, and the cursor it is handed comes back UNCHANGED
- against a journal with rows above the cursor — returns `PollOutcome::Published { count, .. }` and
  advances the cursor to the last published seq

The point is not the individual outcomes; it is that `PollOutcome` **has no variant that ends the
loop**, so 50 idle passes and 50 failures are indistinguishable from 5 in the type system. This test
runs in microseconds and is immune to machine load.

### 2. `the_driver_loop_has_exactly_one_exit` — the residual guard

The one property the step function cannot carry, because it belongs to the driver. Keep exactly ONE
long-window test rather than three: the existing
`tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero` at its current 8000ms.
That arm is the highest-severity one (fall-back-to-0 replays the whole journal onto the live channel)
and it is the only place the driver can still be mutated invisibly.

### 3. `tailer_health_advances_while_polling_and_records_failures`

- after readiness and a published row: `polls_total` has advanced, `consecutive_failures == 0`,
  `last_published_seq` equals the committed seq
- during an induced outage: `consecutive_failures` climbs; after repair it returns to 0
- assert on the COUNTERS, not on timing — this is what makes liveness directly observable instead of
  inferred

## Change

**File:** `crates/services/src/services/event_bus/tailer.rs`

- **Anchor:** the main `loop { ... }` inside `spawn`'s async block.
- **Before:** the loop body inlines `high_water_mark` → `read_range` → per-row `send` → cursor
  advance → `sleep(TAIL_INTERVAL)`, with two `Err` arms that `warn!` and continue.
- **After:** extract the body into

  ```rust
  /// The outcome of ONE poll pass. Deliberately has NO variant that ends the loop:
  /// "give up" is not expressible here, which is the point of this type.
  enum PollOutcome {
      Idle,
      Published { count: usize },
      Failed,
  }

  async fn poll_once(
      pool: &SqlitePool,
      sender: &broadcast::Sender<SequencedEvent>,
      cursor: &mut i64,
      health: &TailerHealth,
  ) -> PollOutcome { /* the current body, verbatim, minus the sleep */ }
  ```

  and reduce the driver to a loop whose ONLY exit is the task being aborted:

  ```rust
  loop {
      let _ = poll_once(&pool, &sender, &mut last_published, &health).await;
      tokio::time::sleep(TAIL_INTERVAL).await;
  }
  ```

  Behaviour must be preserved exactly — same queries, same order, same cursor-advance rules, same
  `warn!` sites, same backoff in the INITIAL mark loop (which is NOT part of this extraction).

- **Anchor:** module top.
- **After:** add the counters, shared with `EventBus`:

  ```rust
  #[derive(Debug, Default)]
  pub struct TailerHealth {
      pub polls_total: AtomicU64,
      pub consecutive_failures: AtomicU64,
      pub last_published_seq: AtomicI64,
  }
  ```

  `spawn` takes an `Arc<TailerHealth>` and updates it each pass: `polls_total` always;
  `consecutive_failures` incremented on `Failed` and reset to 0 otherwise; `last_published_seq` on
  publish.

**File:** `crates/services/src/services/event_bus/mod.rs`

- **Anchor:** `struct EventBus` and `EventBus::new`.
- **After:** hold the `Arc<TailerHealth>` and expose `pub fn tailer_health(&self) -> &TailerHealth`.
  Clones share it, exactly as they share `tailer_handle`.

## Allowed moves

- ONLY: the extraction of the existing loop body into `poll_once`, the `PollOutcome` enum, the
  `TailerHealth` counters and their accessor, and the three tests above.
- The extraction must be **behaviour-preserving**. Do not change a query, a cursor rule, a log site,
  the `TAIL_INTERVAL`, or the initial-mark retry loop's backoff.
- You MAY delete the two 1500ms windows in `tailer_survives_a_transient_read_error` and
  `a_failed_read_does_not_end_the_loop_or_advance_the_cursor`, returning them to a short window —
  but ONLY after test 1 is green and its mutation proof kills. Those windows exist solely to bound
  retry duration, which `PollOutcome` now makes unrepresentable. **Do not touch the 8000ms window**
  (test 2) and **do not lengthen `zero_receivers_does_not_stall_the_cursor`'s 300ms gap**, which the
  ledger records as the only thing pinning `TAIL_INTERVAL` small.
- Every other test in both modules must still pass UNCHANGED. 268 tests is the floor.

## Optional, and explicitly NOT required: virtual time

`tokio::time::pause()`/`advance()` would let a test skip virtual hours and kill a give-up budget of
any size. It is allowed if it works cleanly. **Named hazard:** auto-advance fires when the runtime is
idle, and this code does real sqlx file I/O on a blocking pool — the runtime can look idle while a DB
call is in flight, advancing time early and producing a false green. If you see any nondeterminism,
STOP using it and say so in the ledger; the structural change above is the load-bearing fix and does
not depend on it.

## Mutation proofs (required — record verbatim in the ledger)

Each script must assert its anchor matches exactly once and abort otherwise.

1. Add a terminating variant and return it after N idle passes (i.e. reintroduce panel 6's finding by
   force) → `a_poll_step_can_never_terminate_the_loop` must FAIL. This proves the new test is wired.
2. `break 0` after 10 retries in the INITIAL mark loop →
   `tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero` must FAIL (the 8000ms
   window is retained precisely to keep this kill).
3. `poll_once` returns `Idle` without advancing the cursor after a successful publish →
   an existing cursor test must FAIL.
4. `consecutive_failures` never reset on success →
   `tailer_health_advances_while_polling_and_records_failures` must FAIL.

## STOP triggers

- The extraction cannot be made behaviour-preserving without touching a query or a cursor rule.
- Any pre-existing test in either module goes red and the cause is not a mutation you applied.
- You conclude `PollOutcome` needs a terminating variant for a legitimate reason — that is the whole
  premise of this task; STOP and report rather than adding one.
- The health counters would require a third file (e.g. a metrics crate or a route). They must not.
- `crates/services/tests/normalize_sync_test.rs` fails: that is a KNOWN pre-existing load-sensitive
  flake tracked in `dev-docs/workstreams/normalize-fast-execution-lost-logs-flake/`. Re-run, confirm
  no OTHER test failed, and do not touch that file.

## Done when
`WAI_TYPECHECK_CMD="cargo check --workspace" WAI_TEST_CMD='cargo test -p "$(basename {scope})"' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 016` exits 0
