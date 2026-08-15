---
id: "018"
phase: 2
title: "Close the EventBus startup race by awaiting tailer readiness"
status: ready
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

### 2. `tailer::spawn`'s initial-mark loop MUST become bounded

**File:** `crates/services/src/services/event_bus/tailer.rs`
**Anchor:** the `let mut last_published = loop { match event_journal::high_water_mark(&pool).await
{ ... } }` block inside `spawn`.

That loop currently retries **forever** on error. Awaiting it from `new` without bounding it
converts a background retry into a **startup hang**: a persistent read failure at boot would mean
`EventBus::new().await` never returns and the node never boots, with one `warn!` and no further
output. Today the node boots degraded; this change must not make it fail to boot at all, and must
never fail silently.

**After:** cap the loop at **10 attempts**. On the 10th consecutive failure:

1. `error!` — a distinct message from the existing first-attempt `warn!`, stating that the initial
   mark could not be read and the tailer is starting from 0.
2. Fall back to cursor `0` and proceed into the driver loop.
3. Signal readiness as normal, so `new` always returns in bounded time.

**Why 0 is the safe fallback, and not a fresh hazard.** Starting at 0 republishes journal history
onto the broadcast once. That is safe by contract, not by luck: `subscribe_from`'s Live arm drops
anything with `ev.seq <= state.last` (`mod.rs:200-205`), and a burst large enough to overrun the
channel lands in the `Lagged` refill arm (`mod.rs:207+`), which re-reads from the journal. The
at-least-once contract tolerates duplicates; it does not tolerate the gap this task closes. And in
the degraded case the subsequent `read_range` will fail too, so `consecutive_failures` climbs and
the failure is visible on the task-016 health surface rather than silent.

Record the computed cumulative bound in the ledger. The existing backoff is
`min(1000, 50 * (1 << retry_count.min(4)))`; state the total worst-case delay 10 attempts implies
and confirm it is the boot-delay budget you intend.

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
- The bounded fallback cannot signal readiness without restructuring `spawn`'s return type. Report
  what is in the way.
- Any test outside these three files needs editing to compile. Report which and why — that is a
  hidden production call site the zero-call-site claim above missed.

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
