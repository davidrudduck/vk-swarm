---
id: "019"
phase: 2
title: "Make the seam suite catch tailer cursor defects by giving the tailer a non-zero start mark"
status: ready
depends_on: ["017","018"]
parallel: false
conflicts_with: []
files:
  - "crates/services/tests/event_bus_end_to_end.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: edit
covers_criteria: []
covers_tests: []
---

## Read this before you assume this is a bug fix — it is not

**Nothing is broken and the run is not exposed.** Both mutations this task addresses (M3 and M7,
defined below) are already killed **deterministically** by the `crates/services` lib suite — task
013's own tests — so `cargo test -p services` catches either one every time. The end-to-end suite
also still satisfies the run-level reachability gate's requirement (b) on (b)'s own terms: panel 11
proved it by removing the tailer's `sender.send` and watching three of its five tests die, which is
exactly the "drives the real seam rather than a mock past it" property (b) asks for.

This task is **strengthening**, not defect-closing. Do not prioritise it as though there were a
hole, and do not describe it as one in the ledger.

## What is actually weak

The e2e suite is structurally near-blind to two tailer **cursor** defects:

- **M3** — the tailer skips the first row of each batch while still advancing the cursor. Kills the
  suite **1 run in 4**.
- **M7** — the tailer starts one past its mark (`break mark + 1`), silently dropping the first row
  it would ever publish. Kills the suite **0 runs in 4** — completely invisible, not merely
  under-confirmed.

**The mechanism, which is structural rather than probabilistic.** Every test builds its bus on an
**empty** journal, so the tailer's initial mark is 0. The first row committed is seq 1 — and the
subscriber's `Initializing` arm runs at its first `.next()`, which happens AFTER that commit. So
seq 1 arrives through `subscribe_from`'s own direct journal replay and never touches the broadcast
at all. M7 can only ever damage that one row, so it lands entirely inside the blind spot. M3 hides
in the same place whenever the tailer's poll batches the warm-up row together with the row under
test.

This pre-dates task 018. The retry helper 018 deleted could not have caught either (its liveness
was probe-relative). 018's deletion did not cause it.

## The change — one restructure, already proven

**File:** `crates/services/tests/event_bus_end_to_end.rs`
**Anchor:** `a_committed_row_reaches_a_live_subscriber`.

Move the warm-up commit to **before** `EventBus::new`, so the tailer's initial mark is 1 rather
than 0 and the row under test is the FIRST row the tailer must ever publish.

**Before:**

```rust
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64).await;
    let mut stream: EventStream = bus.subscribe_from(0).unwrap();

    let (warm_seq, _warm_task_id, _warm_project_id) = commit_task_created(&pool).await;
    expect_next_seq(&mut stream, warm_seq, Instant::now() + DEADLINE).await;
```

**After:**

```rust
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    // The warm-up commit lands BEFORE the bus exists, so the tailer's initial mark is 1 and the
    // row under test is the FIRST row it must ever publish. On an empty journal the mark is 0,
    // seq 1 falls inside subscribe_from's own replay window, and a tailer that starts one past
    // its mark (or drops the first row of a batch) is undetectable here. See task 019.
    let (warm_seq, _warm_task_id, _warm_project_id) = commit_task_created(&pool).await;

    let bus = EventBus::new(pool.clone(), 64).await;
    let mut stream: EventStream = bus.subscribe_from(0).unwrap();

    expect_next_seq(&mut stream, warm_seq, Instant::now() + DEADLINE).await;
```

The warm-up row still exhausts the replay window (it is replayed from the journal, since
`subscribe_from(0)` starts below it) — that property is preserved, not traded away.

This works **only because task 018 landed**: `EventBus::new` now awaits readiness, so the tailer's
cursor is established before the test's next commit. Before 018 this shape would have been flaky.

## This was verified before the task was written — do not re-derive it, DO re-confirm it

The orchestrator ran this exact restructure by hand on a machine verified quiet, on top of
`86e85038`, mutating and restoring via `cp` into `.wai-scratch/` with `diff`-verified restores:

```text
M7 (Ok(mark) => break mark + 1), restructured test, 4 runs:
run 1: test result: FAILED. 0 passed; 1 failed; ... finished in 30.20s
run 2: test result: FAILED. 0 passed; 1 failed; ... finished in 30.21s
run 3: test result: FAILED. 0 passed; 1 failed; ... finished in 30.21s
run 4: test result: FAILED. 0 passed; 1 failed; ... finished in 30.21s

CONTROL (unmutated, restructured), full suite, 4 runs:
run 1: test result: ok. 5 passed; 0 failed; ... finished in 2.71s
run 2: test result: ok. 5 passed; 0 failed; ... finished in 2.71s
run 3: test result: ok. 5 passed; 0 failed; ... finished in 2.62s
run 4: test result: ok. 5 passed; 0 failed; ... finished in 2.73s

M3 (skip first row of each batch), restructured, 4 runs: FAILED 4/4,
failing test named: a_committed_row_reaches_a_live_subscriber
```

So: M7 **0/4 → 4/4**, M3 **1/4 → 4/4**, control **4/4 green at unchanged 2.7s**.

**Re-confirm all three yourself.** The orchestrator's numbers are evidence the change is worth
making, not a substitute for your own run — and the orchestrator has had four hypotheses falsified
in this run, three of them about exactly this kind of mutation prediction. If any of the three
disagrees with the table above, that disagreement is the most valuable thing you can report.

## Allowed moves

- ONLY `crates/services/tests/event_bus_end_to_end.rs`. No production change: if the restructure
  appears to need one, that is a STOP.
- Do not weaken any assertion. Every `expect_next_seq` stays exact-seq.
- Do not restructure tests 2–5 to match. Test 2 already commits three rows before subscribing;
  tests 3–5 test different properties. If you believe one of them would ALSO gain a kill from the
  same treatment, report it with the mutation evidence rather than changing it here.

## STOP triggers

- The control run is not 4/4 green, or the suite's wall time grows materially beyond ~2.7s.
- M3 or M7 does not reach 4/4 after the restructure. Report the run table; do not adjust the test
  until it dies.
- Moving the warm-up commit breaks the replay-window-exhaustion property the file header depends on
  (i.e. the warm-up row stops being delivered via replay). That would make test 1's determinism
  argument false and is a STOP, not something to paper over.

## Done when

`WAI_TYPECHECK_CMD="cargo fmt --all -- --check && cargo check --workspace" WAI_TEST_CMD='cargo test -p "$(basename {scope})"' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 019` exits 0, and the three run tables above are reproduced in the ledger.
