---
id: "018"
phase: 2
title: "Close the EventBus startup race by awaiting tailer readiness"
status: passed
depends_on: ["013","016","017"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/event_bus/mod.rs"
  - "crates/services/src/services/event_bus/tailer.rs"
  - "crates/services/tests/event_bus_end_to_end.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: edit
covers_criteria: []
covers_tests: []
---

## Why this task exists

Task 017's implementer hit STOP trigger 3 ("a test fails in a way that indicates a REAL defect in
013/016") and concluded the opposite — that the defect it found is "by design" — then worked around
it with a 90-second retry helper. The orchestrator adjudicated it a **real, production-reachable
at-least-once violation** and opened this task. See the decisions-ledger, task 017, for the full
adjudication.

### The defect, precisely

`EventBus::new` (`mod.rs:83-96`) discards the readiness receiver `tailer::spawn` returns:

```rust
let (tailer, _ready) = tailer::spawn(pool.clone(), _tx.clone(), Arc::clone(&tailer_health));
```

`tokio::spawn` only SCHEDULES the tailer; its initial `high_water_mark()` read resolves later. A row
committed before that read resolves is correctly never broadcast (the tailer's cursor already sits
above it). Separately, `subscribe_from`'s `Initializing` arm (`mod.rs:157-179`) takes its OWN
high-water mark at first poll and replays `(cursor, mark]`, then goes live.

Those two independent mark reads can straddle one commit in **opposite** directions:

| time | event |
|---|---|
| t0 | subscriber's `Initializing` reads mark = N → replay covers `(cursor, N]` |
| t1 | seq N+1 commits |
| t2 | tailer's initial `high_water_mark` resolves = N+1 → cursor starts at N+1 |

Seq N+1 is now **never replayed** (the subscriber is past its window) and **never broadcast** (it is
not above the tailer's cursor). It is permanently lost to that subscriber. That is a direct
violation of the at-least-once contract ADR-0017 rests on.

### It is live, and it was measured — not derived

Ten instrumented runs of the 017 suite on a **verified-idle** machine, counting probe attempts
consumed inside `prove_tailer_is_live` (1 = no race; 2 = one committed event was permanently
dropped):

```text
run 1: probes=[1,1]   run 2: probes=[1,1]   run 3: probes=[1,1]   run 4: probes=[1,1]
run 5: probes=[1,1]   run 6: probes=[1,1]   run 7: probes=[1,2]   run 8: probes=[1,1]
run 9: probes=[1,1]   run 10: probes=[1,1]
```

**1 of 20 probe sites lost an event on an idle box.** Under this session's multi-agent load, task
017's implementer measured the same race as an 8/10 pass rate before it added the retry helper. The
window is not theoretical and it is not narrow enough to ignore.

### Why now, and not "documented as a residual"

- **There are ZERO production call sites today.** `grep -rn 'EventBus::new' --include='*.rs'` returns
  only tests (`mod.rs:249` opens `#[cfg(test)]`; every hit below it is inside it) and the 017 suite.
  Task 014 creates the first real one. Changing the constructor signature is free now and costs
  every consumer later.
- `EventBus::new`'s own doc comment already predicts this: *"Task 014 (startup wiring) likely will
  [need to observe readiness] — at which point this constructor should surface it rather than
  discard it."* This task is that point, moved earlier.
- 017's `prove_tailer_is_live` (a 30 × 3s retry budget) is a workaround whose existence masks the
  defect on every future run. Removing it is this task's acceptance test.

## The change

### 1. `EventBus::new` awaits readiness

**File:** `crates/services/src/services/event_bus/mod.rs`
**Anchor:** `pub fn new(pool: SqlitePool, broadcast_capacity: usize) -> Self`

Make it `pub async fn new(...) -> Self` and `.await` the readiness receiver before returning. After
`new` returns, the tailer's cursor is fixed, so every subsequent commit is strictly above it and is
owed to subscribers — the straddle in the table above becomes unrepresentable.

Update the doc comment: it currently explains why the receiver is dropped. That reasoning is now
withdrawn; say instead what awaiting buys and cite the bounded-fallback rule below.

Every call site is a test. Update them mechanically (`EventBus::new(pool.clone(), 64).await`).
`LocalDeployment::new` — task 014's future call site — is **already `async`**
(`crates/local-deployment/src/lib.rs:156`, `:165` awaits `DBService::new_with_after_connect`), so
this imposes no constraint on 014.

### 2. ~~`tailer::spawn`'s initial-mark loop MUST become bounded~~ WITHDRAWN — bound the WAIT, not the RETRY

> **SUPERSEDED 2026-08-15, before any code was written.** The original section 2 (struck through
> below, preserved so the reversal is on the record) required capping `spawn`'s initial-mark retry
> loop at 10 attempts with a fall-back to cursor 0. **That was an orchestrator error.** Task 018's
> implementer caught it as a STOP before writing anything, and the catch was correct.

> ~~Cap the loop at 10 attempts; on the 10th consecutive failure `error!`, fall back to cursor 0,
> and signal readiness.~~

**Why it was wrong.** `crates/services/src/services/event_bus/tailer.rs:1533-1611` already contains
`tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero`, built in task 013
attempt 8 and cleared by panels 5 and 6, which asserts the **exact opposite**:

```rust
assert!(
    matches!(ready.try_recv(), Err(oneshot::error::TryRecvError::Empty)),
    "the tailer signalled readiness while the journal table was unreadable, so it did not \
     retry the initial high-water mark — it fabricated one"
);
```

Its 8000ms outage window was chosen deliberately to exceed the 5500ms a ten-attempt give-up takes
(`100+200+400+800+800*5`), precisely so that `break 0` after ten retries fails loudly. Mutation
proof (2) in that ledger entry is literally "initial loop `break 0` after 10 retries", recorded as
a kill. **013's adversarial process already tested and rejected the design this task asked for.**

And it was wrong on the merits, not merely in conflict. The hazard that test names is real: a
tailer that signals readiness holding a *fabricated* cursor is worse than one that keeps retrying,
because it then publishes from a mark that was never read. Cursor-0's history replay is survivable;
inventing a cursor is not the kind of thing to trade a hang for.

### 2 (replacement). `EventBus::new` bounds its OWN wait; `spawn` is not touched

**File:** `crates/services/src/services/event_bus/mod.rs` only.

**`crates/services/src/services/event_bus/tailer.rs`'s `spawn` function must not change at all.**
Its retry-forever semantics are correct and stay. Do not edit
`tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero`; it must stay green,
untouched, as written.

Instead, `new` races the readiness receiver against its own timeout:

1. `tokio::time::timeout(READY_TIMEOUT, ready_rx)` where `READY_TIMEOUT` is a new module constant
   in `mod.rs`, set to **10 seconds**.
2. On success: the cursor is fixed and the race is closed — the normal path, and the one the
   1-in-20 measurement lives on.
3. On timeout: `error!` (state that the tailer has not established its cursor within the budget,
   that it is still retrying, and that events committed in this window may not be broadcast), then
   return the bus anyway.

**What this buys and what it does not.** It closes the race whenever readiness resolves, which is
whenever the journal is readable — every case measured. In the pathological case (journal
unreadable for a full 10s at boot) it degrades to exactly today's behaviour, which is a strict
improvement over today rather than a regression, and it is now **loud** where today it is silent.
Boot cannot hang. No existing invariant is disturbed.

Record the chosen 10s in the ledger with your reasoning: the tailer's backoff caps at 1000ms per
retry, so 10s is roughly 12 attempts, and a journal unreadable that long at startup is a node with
larger problems than event delivery.

**REQUIRED test — `new` returns even when readiness never fires.** Rename the `event_journal` table
away (follow the `ALTER TABLE event_journal RENAME TO event_journal_hidden` pattern the existing
test uses), call `EventBus::new`, and assert it RETURNS rather than hanging. To keep this test fast,
add a private `async fn new_with_ready_timeout(pool, capacity, ready_timeout)` that `new` delegates
to, and drive the test through it with a short timeout. It is a private helper reachable from the
colocated `#[cfg(test)] mod tests` — do **not** widen the public API for it. Rename the table back
and abort the tailer before the test ends.

Mutation proof for this test: make `new` await readiness unconditionally (no timeout) → the test
must hang and fail, not pass.

### 3. Delete `prove_tailer_is_live` from the 017 suite

**File:** `crates/services/tests/event_bus_end_to_end.rs`

Its entire justification is the race this task closes. Replace each call with a plain
`commit_task_created` + `expect_next_seq` at `WARM_LIVE_DEADLINE`, and delete
`PROBE_ATTEMPT_WINDOW`, `PROBE_ATTEMPTS`, and every doc comment that explains the workaround. Test
2's declared residual in the ledger (its handoff "is not structurally immune to the same race, just
improbable") must be revisited and retired if this change makes it immune — say so explicitly
either way.

**Do not weaken any assertion while doing this.** Every `expect_next_seq` stays exact-seq; the
warm-up commits that provably exhaust the replay window stay exactly as they are.

## Allowed moves

- ONLY the three files above. If closing the race appears to require changing `subscribe_from`'s
  state machine, that is a STOP — report the reasoning; do not redesign the replay handoff here.
- No `#[ignore]` on any test. No widening of any existing deadline to make something pass.

## STOP triggers

- Awaiting readiness in `new` does **not** remove the flake measured in the acceptance bar below.
  That means the straddle analysis above is wrong or incomplete — report the counterexample with the
  captured run output rather than adding a retry anywhere.
- Any change to `mod.rs` appears to require editing `spawn` or any existing test in `tailer.rs`.
  Section 2's replacement exists precisely so nothing in `tailer.rs` has to move — if something
  does, STOP and say what.
- Any test outside these three files needs editing to compile. Report which and why — that is a
  hidden production call site the zero-call-site claim above missed.
- **You find another existing invariant this task contradicts.** The original section 2 reversed a
  panel-cleared decision from task 013 and the implementer caught it as a STOP before writing code.
  That was the correct move and it is welcome again — a task file is the orchestrator's reasoning,
  not ground truth, and it has been wrong repeatedly in this run. Read the ledger before you build,
  and STOP on a conflict rather than picking a side.

## Acceptance bar (this is the proof, not a unit test)

This race is probabilistic; a single mutation kill is not available and must not be faked. The
evidence is statistical and both halves are required:

1. **Fixed:** with readiness awaited and `prove_tailer_is_live` deleted, run
   `cargo test -p services --test event_bus_end_to_end` **30 times** on a machine verified quiet
   (`pgrep -x cargo` empty immediately before). Require **30/30 green**. Paste the run table.
2. **Counterfactual:** revert ONLY the `new`-awaits-readiness change (keep the helper deleted), run
   the same 30. Require **at least one failure**. At the measured ~1-in-20 per probe site with 2
   sites per run, 30 runs is ~60 exposures — expect roughly 3. Paste the run table and the failure
   text verbatim.

   Back the counterfactual out with `cp` from a `.wai-scratch/` backup and `diff`-verify the
   restore. **Never** `git checkout`/`restore`/`stash`/`reset`/`clean` in any form.

If the counterfactual comes back 30/30 green, the acceptance bar has proven nothing and you must
say so rather than banking half of it — report it and stop.

## Done when

`WAI_TYPECHECK_CMD="cargo check --workspace" WAI_TEST_CMD='cargo test -p "$(basename {scope})"' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 018` exits 0

---

## REQUIRED — added after panel 11 on task 017

Panel 11 returned `NO CITED DISSENT` on task 017 with six non-blocking findings. Four of them
(F1, F3, F5, F6) have a **single shared root cause: `prove_tailer_is_live` itself.** This task
already deletes that helper, so this section states the hypothesis that deleting it resolves all
four — and REQUIRES you to prove it empirically rather than inherit the claim.

**Do not take the hypothesis on trust. It is the orchestrator's reasoning, and the orchestrator has
been wrong about exactly this class of derivation repeatedly in this run (see the ledger for tasks
013 and 016 — four separate "this mutation will be killed" predictions that were falsified by
counterfactual).** If a mutation below still escapes after the helper is gone, that is a finding,
not a failure to explain away.

### The three mutations that ESCAPED the 017 suite

All three were run by panel 11 at commit `2ebd5b01` in a detached worktree. Each is killed
deterministically by the `crates/services` **lib** suite, so no defect escapes the run as a whole —
but each escaped the end-to-end suite, which is the run's evidence for reachability gate (b).

**M3 — tailer skips the first row of each batch, cursor still advances** (`tailer.rs:85`):

```rust
for (m3_i, seq_ev) in seq_events.into_iter().enumerate() {
    if m3_i == 0 { *cursor = seq_ev.seq; continue; }
```

Four consecutive runs of the e2e suite on a quiet box:

```text
run 0: FAILED. 4 passed; 1 failed   (test 1)
run 1: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.66s
run 2: FAILED. 4 passed; 1 failed   (test 4)
run 3: FAILED. 4 passed; 1 failed   (test 4)
```

Run 1 escaped the entire suite. Mechanism: the suite asserts *a named row arrives*, never *every
committed row arrives*, and whether the dropped row is one a test names is decided at runtime by
the tailer's batch boundaries.

**M7 — tailer drops the first row it would ever publish** (`tailer.rs:164`, `Ok(mark) => break mark + 1`):
e2e `ok. 5 passed` x 2 runs. Lib: `FAILED. 16 passed; 15 failed`.

**M8 — tailer fabricates `project_id = Uuid::nil()`, seq and `task_id` honest**:
e2e `ok. 5 passed`. Lib: `FAILED. 16 passed; 15 failed`.

### The hypothesis you must test

Deleting `prove_tailer_is_live` should kill M3 and M7 deterministically, because the helper is the
mechanism behind both escapes:

- **M3.** The helper commits up to 30 probe rows in a tight loop, which is what lets multiple rows
  land in one tailer batch and gives a "skip the first of each batch" mutation somewhere to hide.
  With the helper gone, each test commits rows one at a time with an `await` between, so batches
  are single-row and every row is dropped by the mutation.
- **M7.** The helper's liveness is *probe-relative*: a dropped first row merely rebases the frame
  and the next probe succeeds. This is the same pattern `tailer.rs`'s `await_ready` doc records as
  the reason `probe_until_live()` was deleted in task 016 — *"which is why a tailer that silently
  DROPPED the first row it would ever publish passed the entire suite."* Panel 11's F3 is that this
  retired pattern was reintroduced by 017 without noticing the sibling ledger. Absolute-seq
  assertions after this task's fix should restore the deterministic kill.
- **F5 and F6** are textual consequences: F5 (the header's "at most 10 events per test" is false —
  30 probes make it ~34) becomes true again once `PROBE_ATTEMPTS` is gone; F6 (the helper's
  `_ => break` swallows `Ok(Some(Err(e)))`, so a journal error is misreported as "the tailer never
  went live") disappears with the code that contains it.

**REQUIRED: after the fix, re-run M3, M7 and M8 and paste the verbatim result of each.**

- **M3 and M7 must each fail the e2e suite on 4 consecutive runs (4/4).** Anything less than 4/4 is
  a surviving residual and must be reported as such — do NOT round 3/4 up to "killed".
- **M8 requires a code fix, not just the deletion** — see below — so run it after that fix.

### F4 — `assert_task_created_body` must assert `project_id`

`event_bus_end_to_end.rs:146` matches `NodeEvent::TaskCreated { task_id, .. }`, discarding
`project_id`. The sibling assertion in `tailer.rs:249-250` deliberately checks both, with the
recorded reason: *"so a mutation that fabricates only one of the two fields has nowhere to hide
either."* 017 diverged from its sibling without recording why.

**REQUIRED:** thread the committed `project_id` through `commit_task_created` and assert it in
`assert_task_created_body`. Then re-run M8; it must fail the e2e suite 4/4.

### F2 — two false comments claiming a property the test cannot observe

`event_bus_end_to_end.rs:272-274` claims test 2 proves *"no duplicate across the boundary"*, and
lines 382-383 claim test 4 proves *"no stray replay of the two pre-restart events"*. Both are
false: `subscribe_from`'s Live arm drops anything with `ev.seq <= state.last` (`mod.rs:200`), so a
duplicate is consumed before the stream ever yields it. Panel 11 proved it — publishing every row
twice leaves the e2e suite `ok. 5 passed` x 3 runs while the lib suite reports
`the bus delivered the single committed row at seq 2 2 time(s)`.

**REQUIRED:** correct both comments to state what the assertion actually proves, and name
`the_bus_publishes_a_committed_row_exactly_once` (in `event_bus/mod.rs`) as where the exactly-once
property genuinely lives. **Do not attempt to make the e2e suite observe duplicates** — that would
mean reaching around `subscribe_from`'s dedupe, which is the hand-driving this suite forbids. This
is a comment fix, not a coverage gap.

### What is NOT in scope here

Panel 11 could not construct a mutation that only test 3 (`a_rolled_back_transaction_reaches_no_subscriber`)
kills, and stated plainly that it also cannot prove none exists. Its rollback half is unfalsifiable
by construction: no delivery-path mutation can make a non-existent journal row appear. **Leave test
3 alone.** It is cheap, it is honest about what it checks, and removing it would be trading a
possibly-redundant test for a definitely-smaller safety net.

---

## REQUIRED — attempt 2, after panel 12 rejected attempt 1

Attempt 1 was **rejected on one blocking finding**. The implementation is sound and stays: `new`
async, the bounded wait, `READY_TIMEOUT`, the helper deletion, F4's `project_id`. Do NOT redo any of
it, and do NOT re-run the 30-run acceptance bar — it passed and nothing below invalidates it.

Four fixes. All are small. The blocking one is first.

### 1. BLOCKING — finish the F2 correction that the ledger records as already done

Each of the two comments had **two** false clauses. The duplicate half was corrected; the
stray-replay half was left verbatim:

```text
event_bus_end_to_end.rs:253   // further — in particular no belated replay of seqs 1..=4 — arrives after the handoff. The
event_bus_end_to_end.rs:369   // No stray replay of the two pre-restart events sneaks in afterward. A duplicate of either
```

Both are false by the **same rule the corrected half of the same comment cites two lines earlier** —
`subscribe_from`'s Live arm drops `ev.seq <= state.last` (`mod.rs:200`):

- `:253` — at `assert_quiet`, `state.last` is 4, so seqs 1..=4 are dropped before the stream yields
  them. The silence cannot prove a belated replay of them did not happen.
- `:369` — the subscriber is `bus2.subscribe_from(high_water)`, so `last` is 2 from the first poll
  and seqs 1-2 are below the cursor by construction.

Delete or qualify each retained clause so the comment states only what the assertion actually
proves. Then say plainly what the silence DOES prove, if anything beyond "no event with a seq above
the cursor arrived".

**This is blocking because the ledger recorded it complete.** The defect is the false completion
claim, not the comment. When you correct the code, also correct the ledger's `### F2` entry — it
names the stray-replay clause as corrected while its own justification covers only the duplicate
half.

### 2. `mod.rs:974` was falsified by this task's own change

```text
974:        // `EventBus::new` drops the tailer's readiness receiver, so no happens-before edge is
```

`new` no longer drops it. That false premise is the whole stated justification for
`wait_until_tailer_publishes` (`mod.rs:824`), a 10-probe retry loop — the same pattern this task
deleted from the e2e suite, surviving in the lib suite on a rationale this task invalidated.

Correct the comment. Then **assess whether the helper is still needed** now that `new` awaits
readiness, and record your answer either way. If it can be replaced with an exact-seq assertion,
do it and prove the mutation still dies. If it cannot, say precisely why — that reason is worth
more than the removal.

### 3. Pin that the wait ENDS when readiness fires, not merely that it ends

The required hang-proof test pins "returns within budget". It does not pin "the wait ends **because**
readiness fired". Panel 12 proved the gap by replacing the awaited future with `pending()` — never
observe readiness, always sleep the full budget:

```text
test ...new_returns_even_if_the_tailer_never_signals_readiness ... ok
whole lib suite: ok. 32 passed; 0 failed; ... finished in 44.64s   (baseline 12.51s)
```

Everything green. In production every `EventBus::new()` would silently cost the full 10s and every
health surface would read green — the exact green-while-degraded class task 016 exists to close,
reappearing one layer up.

**REQUIRED:** an assertion in a healthy-pool test that `new` returns in well under `READY_TIMEOUT`.
The lib suite runs 32 tests with many bus constructions in ~12.5s total, so a real construction is
milliseconds; pick a bound with generous headroom over scheduling noise but far below 10s, and
justify the number in the ledger from a measurement rather than a guess.

**Mutation proof required:** `timeout(ready_timeout, pending())` must make this test FAIL. Paste it.

### 4. Correct the ledger's self-contradiction about test 5

The M8 section calls `every_event_variant_survives_the_full_round_trip` both "a stricter check that
was already catching M8 before F4" and one of the tests that "pass unaffected". Both cannot be true.
The code settles it: test 5 subscribes AFTER committing all nine variants
(`event_bus_end_to_end.rs:443`), so every variant arrives via `Initializing` direct replay and never
touches the tailer — it cannot catch a tailer-publish mutation in either direction.

Fix the ledger text. **Do NOT restructure test 5** — that is a coverage question, it shares a
mechanism with the M3/M7 residuals, and it belongs to task 019's territory, not here. Record it as
an observation for 019 to weigh.

## Verification for attempt 2

- `cargo test -p services` exit 0, `cargo fmt --all -- --check` exit 0,
  `cargo clippy -p services --all-targets --all-features -- -D warnings` exit 0,
  `cargo check --workspace` exit 0.
- The mutation proof for item 3, verbatim.
- If you touched `wait_until_tailer_publishes` under item 2, the mutation proof that its replacement
  still kills what it killed.
- Do NOT re-run the 30-run acceptance bar.
