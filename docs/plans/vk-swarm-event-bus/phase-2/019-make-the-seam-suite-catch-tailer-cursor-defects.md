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
  - "crates/services/src/services/event_bus/tailer.rs"
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

---

## SECONDARY — two stale-anchor fixes inherited from task 018 (panel 13, F13-1 and F13-2)

**These are unrelated to the mutation work above.** They are here because 019 already owns the e2e
suite and because CLAUDE.md forbids carrying a finding into a later session, not because they belong
to the same thought. Keep them in a separate commit-sized chunk of your work and do not let them
influence the mutation measurements. `crates/services/src/services/event_bus/tailer.rs` has been
added to this task's `files:` solely for fix 2 — **change nothing in it but that one comment line.**

### Fix 1 — two comments cite a line number that task 018 moved

```text
event_bus_end_to_end.rs:258   // with `ev.seq <= state.last` (`event_bus/mod.rs:200`), ...
event_bus_end_to_end.rs:390   // (`event_bus/mod.rs:200`). Seqs 1-2 are below the cursor ...
```

`mod.rs:200` WAS the Live-arm dedupe before task 018. Attempt 1 made `new` async and added the
timeout block, moving it. Verified at this commit:

```text
$ grep -n "if ev.seq > state.last" crates/services/src/services/event_bus/mod.rs
254:                                    if ev.seq > state.last {
```

Repoint both to `mod.rs:254`. **Verify the line yourself before writing it** — if further work has
moved it again, the number in this task file is as stale as the one it replaces. That is the whole
point of the finding.

### Fix 2 — `tailer.rs:150` still asserts the premise task 018 falsified

```text
150:/// `let _ = ...` deliberately — a caller that drops the receiver (as `EventBus::new` does) must
```

`EventBus::new` has awaited the readiness receiver since `86e85038`; it does not drop it. This is
the same class as the `mod.rs:974` comment 018 fixed, one file over.

Correct the parenthetical so it no longer claims `EventBus::new` drops the receiver. The `let _ =`
defensive-send rationale itself is still valid and should survive — a caller COULD drop the
receiver, and the tailer must not panic if one does. Rewrite it as the hypothetical it now is
rather than deleting it.

**Nothing else in `tailer.rs` may change.** Panel 13 verified it byte-identical across the whole of
task 018 (`git diff 86e85038~1 587322cd -- .../tailer.rs` empty), and `spawn`'s retry-forever
semantics plus `tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero` are
fenced by task 018's own record. Prove your diff touches only the comment.

### Why fix 2 was not done in 018 itself

Task 018's replacement section 2 stated "`tailer.rs`'s `spawn` function must not change at all", and
a byte-identical `tailer.rs` was an attempt-2 deliverable that panel 13 confirmed. Touching it there
would have invalidated a verified property mid-task. It moves here instead.

**A note on how this was missed, worth reading before you write your own sweep.** Attempt 2 did run
a grep sweep for stale premises, and its pattern was `probe\|wait_until_tailer_publishes\|drops the
tailer\|readiness receiver`. `tailer.rs:150` reads "drops the **receiver**" — the sweep would have
missed this line even had `tailer.rs` been in scope. When you sweep, match on the CONCEPT with
several phrasings, and say in the ledger which patterns you used so the next reader can judge the
coverage rather than trust it.
