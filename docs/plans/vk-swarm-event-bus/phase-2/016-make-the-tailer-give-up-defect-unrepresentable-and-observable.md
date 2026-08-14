---
id: "016"
phase: 2
title: "Make the tailer give-up defect unrepresentable, and the tailer observable"
status: rejected
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
- ~~You MAY delete the two 1500ms windows in `tailer_survives_a_transient_read_error` and
  `a_failed_read_does_not_end_the_loop_or_advance_the_cursor`~~ **WITHDRAWN after attempt 1 — this
  permission was based on a false premise and the panel disproved it. `PollOutcome` constrains
  `poll_once`; it does NOT constrain the driver loop, which is where give-up still lives. See
  "REQUIRED after attempt 1" at the foot of this file for what replaces those windows.**
  **Do not touch the 8000ms window**
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

## REQUIRED after attempt 1 — the panel refuted this task's central premise

**The orchestrator's reasoning in the "Allowed moves" section above was wrong, and the permission it
granted to delete the two 1500ms windows must be treated as withdrawn.**

The claim was: *"Those windows exist solely to bound retry duration, which `PollOutcome` now makes
unrepresentable."* `PollOutcome` makes give-up unrepresentable **inside `poll_once`**. It does not
constrain the **driver loop**, and the driver is where the loop actually lives. Conflating the two was
a conceptual error, not a detail.

The panel proved it with the same mutation producing opposite verdicts:

| suite state | driver gives up after 10 consecutive failures |
|---|---|
| post-attempt-1 (225ms window) | `test result: ok. 270 passed; 0 failed` |
| 1500ms window restored | `panicked at tailer.rs:802:9: tailer should survive the transient read error` |

The measured detection floor fell from ~20 poll passes to **4** — a 5x regression on the most-attacked
property in this file. `a_poll_step_can_never_terminate_the_loop` cannot see it, because it drives
`poll_once` directly with no spawned task and no driver.

### Fix all three findings together — they share one solution

Do NOT simply restore the two 1500ms sleeps. Restoring fixed wall-clock windows reinstates the exact
weakness the ledger's declared residual describes: a budget above the window still passes, and every
run pays the full sleep on every machine. **Use the health counters as the observable instead.**

1. **Driver liveness, waiting on a counter rather than a clock.** In both
   `tailer_survives_a_transient_read_error` and
   `a_failed_read_does_not_end_the_loop_or_advance_the_cursor`: induce the outage, then poll
   `health.consecutive_failures` until it reaches **at least 25**, with a generous deadline as a
   safety net — not a fixed sleep. Then repair and assert a row committed after the repair is
   published with its absolute seq. This kills any driver give-up budget below 25, returns as soon as
   the counter arrives rather than burning a fixed 1500ms, and makes the counters load-bearing instead
   of decorative.

2. **F1 — `EventBus::tailer_health()` is wired to nothing verifiable.** Handing the tailer a
   different `Arc` than the accessor returns survives the whole suite:
   ```text
   MUTATION APPLIED: EventBus hands the tailer a DIFFERENT Arc than tailer_health() returns
   test result: ok. 270 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 11.22s
   ```
   The accessor has zero callers and zero tests; the only health test calls `tailer::spawn` directly
   and holds its own `Arc`. Add a test at the **`EventBus`** layer asserting that
   `bus.tailer_health().polls_total` advances while the bus's own tailer runs. Without it, a `/health`
   surface built on this accessor would report zeros forever — reproducing the green-while-dead mode
   this task exists to remove.

3. **F3 — the counters are self-reporting.** The health test publishes exactly one row, so every
   counter is indistinguishable from the literal `1`. Both of these survive:
   ```text
   MUTATION APPLIED: polls_total frozen at 1, never counts
   test result: ok. 270 passed; 0 failed; ... finished in 11.19s
   MUTATION APPLIED: last_published_seq hardcoded to 1
   test result: ok. 270 passed; 0 failed; ... finished in 11.19s
   ```
   Assert them AS COUNTERS: publish several rows across several passes; require `polls_total` to
   climb strictly between two observations, and `last_published_seq` to equal the LAST seq, not `1`.
   A liveness signal that cannot be falsified is worse than none.

4. **Initialise `last_published_seq` to the resolved initial mark**, not `0`. On a non-empty journal
   it currently reads `0` — indistinguishable from "nothing ever published" — until the first
   post-start publish.

### Mutation proofs (required; anchor-guarded, `assert s.count(OLD) == 1`)

1. Driver gives up after **10** consecutive failures → both driver-liveness tests must FAIL.
2. `EventBus` hands the tailer a different `Arc` than `tailer_health()` returns → the new
   EventBus-layer health test must FAIL.
3. `polls_total` frozen at 1 → the counter test must FAIL.
4. `last_published_seq` hardcoded to 1 → the counter test must FAIL.
5. The attempt-1 proofs must still kill: a terminating `PollOutcome` variant, and `break 0` after 10
   retries in the initial-mark loop.

### What attempt 1 got RIGHT and must be preserved
The panel verified the extraction line by line and found **no semantic change**: identical `debug!`
value and firing point, `count` computed before the publish loop, cursor advanced immediately after
`send` regardless of its result, both error arms preserving their `warn!` text and not touching the
cursor, and the initial-mark loop and `TAIL_INTERVAL` untouched. Keep all of it. The defect is in the
tests, not the extraction.

## REQUIRED after attempt 2 — I fixed one of the two paths, and the fix introduced a counter that can lie

Attempt 2 did what it was asked and the panel confirmed it: a failure-path give-up budget below 25
dies at the counter-wait site. But the brief was wrong in the same way, one level down.

**Orchestrator error (the sixth).** Panel 7 established that `PollOutcome` constrains `poll_once`, not
the driver. I applied that insight to the FAILURE path only, by specifying a wait on
`consecutive_failures` — a counter that moves *only* on `PollOutcome::Failed`. Panel 6's original
finding was about the **idle** path. Same loop, same shape, untouched:

| driver-local give-up in `spawn`'s loop | result |
|---|---|
| after 10 consecutive **Failed** | DIES at the counter wait |
| after 40 consecutive **Idle** (3.0s of quiet) | **`272 passed; 0 failed`** |
| after 100 consecutive **Idle** (7.5s) | **`272 passed; 0 failed`** |
| adaptive **Idle** backoff to 60s (loop never ends) | **`272 passed; 0 failed`** |

The backoff shape is the worst of them: the loop never ends, so `!is_finished()` can never fire and
every counter keeps climbing — 800x slower. Blast radius needs no fault at all: ~3s of journal quiet
is the normal state of an idle node, and phase 4/5 SSE consumers stall while every health surface
reads green.

**And the fix created a new hazard.** The liveness tests now depend on `consecutive_failures` being
correct, which a wall clock never did. Over-counting production code defeats them:

```text
MUTATION APPLIED: consecutive_failures over-counts 3x per failed pass
test result: ok. 272 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 11.02s
```

`await_consecutive_failures` then returns after **9** real failing passes instead of 25. The advertised
detection floor — the entire justification for attempt 2 — silently becomes 9, and the only visible
symptom is that the tests got *faster* (1.12s vs 2.33s).

### The fix: one observable covers both paths, and the counters get pinned exactly

**1. Switch the liveness observable from `consecutive_failures` to `polls_total`.** It increments on
EVERY pass regardless of outcome, so a single mechanism covers the idle path, the failure path, and
adaptive backoff. Add `await_polls_climb(health, +20, deadline)` and use it in **two** driver tests:

- **idle cadence**: tailer running against a quiet journal — `polls_total` must climb by 20
- **faulted cadence**: tailer running with the journal unreadable — `polls_total` must climb by 20,
  then repair and assert the post-repair row publishes at its absolute seq

**Derivation, so the numbers can be checked rather than trusted** (`TAIL_INTERVAL = 75ms`):

| | value |
|---|---|
| 20 passes, healthy | 1.50s |
| 20 passes, 2x load | 3.00s |
| 20 passes, 4x load | 6.00s |
| deadline **8s** catches any per-pass interval | **≥ 400ms** |

So: **20 climbs, 8s deadline.** Healthy returns in ~1.5s; survives 4x load with 2s of margin; a
give-up never arrives and fails at the deadline; a backoff to 400ms or worse fails.

**DECLARED RESIDUAL, stated rather than discovered later:** an adaptive backoff *below* ~400ms is not
distinguishable from a loaded machine by this test, and is not caught. That is a deliberate trade —
perfect discrimination of "slower cadence" from "slow machine" is not achievable by timing alone. The
uncaught case is a latency regression, not silent death; the give-up case, which is silent death, is
caught cleanly because the counter freezes forever.

**2. Pin the counters EXACTLY, synchronously, with zero sleeps.** This removes F2's circularity at the
root: drive `poll_once` directly a known number of times and assert exact equality, not `>=`.

- K calls against a quiet journal → `polls_total == K` exactly
- K calls against an unreadable journal → `consecutive_failures == K` exactly (kills `fetch_add(3)`)
- one successful pass after failures → `consecutive_failures == 0`
- **publish a MULTI-ROW batch in one pass** → `last_published_seq` equals the batch's LAST seq. This
  is F3: every health-observing test today publishes one row at a time, so `count == 1` and first ==
  last on every pass, and storing the batch's FIRST seq survives the whole suite.
- a failed pass must still increment `polls_total` (F4: the contract says "whatever their outcome",
  and moving the increment into the `Ok(mark)` arm currently survives).

Once the counters are pinned exactly, the cadence tests may rely on them.

**3. Keep the existing `consecutive_failures >= 25` waits** — they kill a failure-path budget below 25
at a named site, which the cadence test does not do as precisely. They are no longer the ONLY driver
guard, which is the point.

### Mutation proofs (anchor-guarded, `assert s.count(OLD) == 1`) — all must kill
1. Driver returns after 40 consecutive `Idle` passes → the idle-cadence test must FAIL.
2. Driver sleeps 60s after 40 idle passes (loop never ends) → the idle-cadence test must FAIL.
3. Driver returns after 10 consecutive `Failed` → the faulted-cadence test AND the existing counter
   waits must FAIL.
4. `consecutive_failures` over-counts 3x per failed pass → the exact-count test must FAIL.
5. `last_published_seq` stores the batch's FIRST seq → the multi-row exact test must FAIL.
6. `polls_total` not incremented on a failed pass → the exact-count test must FAIL.
7. Attempt 1 and 2's proofs must still kill: terminating `PollOutcome` variant; `break 0` after 10
   initial-mark retries; different `Arc` in `EventBus`; `polls_total` frozen at 1.

### Recorded as accurate, do not re-attack
- A give-up budget of exactly 25 DIES (at `is_finished()`, not the counter wait — the ledger's claim
  that failures always name the budget is true only *below* 25; that is documentation imprecision, not
  a defect).
- `assert_nothing_published` treating `Lagged` as a panic is sound: capacity 64, a handful of events,
  nothing consumed between fault and `try_recv`, so `Lagged` is unreachable.
- `last_published_seq` initialised to the resolved mark is the deliberate cursor semantic, stored
  before readiness so observers see a consistent pair.
- The `polls_total >= before + 3` bound is correctly justified (the increment precedes the read).
