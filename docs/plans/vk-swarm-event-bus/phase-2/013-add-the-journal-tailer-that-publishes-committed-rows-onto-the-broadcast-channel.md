---
id: "013"
phase: 2
title: "Add the journal tailer that publishes committed rows onto the broadcast channel"
status: passed
depends_on: ["005"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/event_bus/tailer.rs"
  - "crates/services/src/services/event_bus/mod.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
**File:** `crates/services/src/services/event_bus/tailer.rs` (colocated
`#[cfg(test)] mod tests`), using `db::test_utils::create_test_pool_with_migrations()`.

1. `tailer_publishes_committed_rows_in_seq_order` — append and COMMIT three journal rows, run one
   tail pass, assert a subscriber receives seqs 1,2,3 in that order.
2. `tailer_never_publishes_a_rolled_back_row` — open a transaction, append, ROLL BACK; run a tail
   pass; assert the subscriber receives nothing. Then commit a different row and assert only that one
   arrives. **This is the structural replacement for the old `broadcast_only_after_commit` test.**
   The previous version tried to broadcast inside a transaction and expected rollback to retract the
   message — impossible, because `tokio::sync::broadcast::Sender::send` is not transactional; once
   sent, every receiver has it. Here the guarantee is structural instead: an uncommitted row is not
   readable, so it cannot be tailed.
3. `tailer_does_not_republish_across_passes` — run two passes with no new rows; assert the second
   publishes nothing (the cursor advanced).
4. `tailer_resumes_from_its_high_water_on_restart` — publish 3, drop the tailer, construct a new one,
   append 2 more, run a pass; assert only the 2 new rows are published.
5. `tailer_survives_a_high_water_mark_failure` — exercises the OUTER arm. `pool.close()` is NOT
   acceptable (irreversible in sqlx: there is no "then succeed" half). **`chmod 000` is NOT acceptable
   either (2026-08-12):** POSIX checks permissions at `open(2)`, not at read, and
   `create_test_pool_with_migrations` sets `.min_connections(1)` (`crates/db/src/test_utils.rs:118`),
   so the pool holds an open fd and reads — and writes — keep succeeding after `chmod 000` on the db,
   `-wal` and `-shm` alike. Verified empirically. **Pool exhaustion is also wrong here for test 7**
   (see below) though it would work for this one. Induce the fault with a reversible schema change:
   `sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden").execute(&pool)`, then
   rename back. Both queries then fail with "no such table" and the outer arm is taken; SQLite
   auto-reprepares on `SQLITE_SCHEMA`, so the same pooled connection recovers after the rename-back
   (verified). Assert THREE things: (a) during the outage, a row committed at that time is NOT
   published — this proves the fault actually fired, without which the test is vacuous; (b) the tailer
   task is still running (`!handle.is_finished()`); (c) after the rename-back, that same row IS
   published. (c) is what proves property 3's "do NOT advance `last_published` on a failed read" — had
   the cursor advanced past the outage, the row would be lost forever.
6. `zero_receivers_does_not_stall_the_cursor` — NEW (2026-08-12). Property 2 says `send` errors are
   ignored deliberately and `last_published` advances regardless, because zero receivers is the normal
   idle state of a node. Every attempt-1 test subscribed BEFORE publishing, so `send` never returned
   `Err` and a mutation that advanced only on `send` success survived the whole suite.
   **Required shape, all four points binding:**
   (a) Create the channel as `let (tx, _) = broadcast::channel(64);` — **NOT** `let (tx, _rx) = ...`,
       the idiom used everywhere else in this file. `_rx` is a live binding for the whole test, so
       `rx_cnt` is 1, `send` succeeds, and the test passes under the mutation too — vacuous. Dropping
       the receiver does not permanently close the channel: tokio resets `tail.closed` when
       `rx_cnt == 0` on the next `subscribe()`, so step (c) works.
   (b) Spawn the tailer FIRST, so its cursor is below seq 1, and only THEN commit rows 1-3. If the
       tailer is spawned after the commits it starts at mark 3, never attempts those sends, and the
       mutation survives again.
   (c) Let at least one pass run (sleep > `TAIL_INTERVAL`), THEN `tx.subscribe()`, THEN commit one
       more row.
   (d) Assert the FIRST event received has the new row's seq, and that no further event arrives.
       "the new seq is among those received" is NOT sufficient — it passes under the mutation.
   Why this discriminates: `broadcast::Sender::send` returns `Err` WITHOUT buffering when
   `rx_cnt == 0`, and a late subscriber starts at the current tail. Under the correct implementation
   the cursor has already advanced past 3, so only the new row is ever sent; under the mutation the
   cursor is still 0 and the next pass RE-sends seqs 1,2,3 after the subscriber attaches, making its
   first message seq 1.
7. `a_failed_read_does_not_end_the_loop_or_advance_the_cursor` — NEW (2026-08-12). Property 3 has two
   halves and attempt 1 tested neither for the `read_range` call. Mutations that returned from the loop
   on a read error, and that advanced `last_published` by 1000 on a failed read, BOTH survived the
   suite. **Do NOT restructure the loop to fix this** — nesting is not the obstacle; the obstacle is
   that every fault tried also broke `high_water_mark`, so the outer arm tripped first. Use a fault
   that hits `read_range` ONLY: corrupt a committed row's payload with
   `UPDATE event_journal SET payload = '{not json' WHERE seq = ?`. `read_range` fails at
   `serde_json::from_str` (`crates/db/src/models/event_journal/queries.rs:55` →
   `EventJournalError::Serde`) while `high_water_mark` (`:67`, `SELECT COALESCE(MAX(seq), 0)`) is
   unaffected. The column is `payload TEXT NOT NULL` with no `CHECK json_valid`
   (`crates/db/migrations/20260812000000_add_event_journal.sql:12`), so this is storable; verified.
   Repair it with `serde_json::to_string(&event)` for the same event. Assert THREE things: (a) while
   the payload is corrupt, nothing is published — proves the fault fired; (b) the tailer task is still
   running; (c) after the repair, that seq IS published. (c) kills both surviving mutations: a loop
   that returned would never publish it, and a cursor that jumped by 1000 would read an empty window
   forever.

**Every fault-injection test above must assert the outage window is observably silent BEFORE the
repair.** A fault that silently fails to fire otherwise yields a passing test — which is exactly how
the `chmod 000` version would have shipped green.

**Waiting must be DEADLINE-BASED, not fixed sleeps (added 2026-08-12 after attempt 2 shipped a flaky
suite).** Attempt 2 paired `sleep(200ms)` with `timeout(50ms, recv())` against a 75ms
`TAIL_INTERVAL`. Run alone it passed; run inside the full 261-test `-p services --lib` suite it failed
1-in-3, with two different tests flaking across four consecutive runs
(`tailer_publishes_committed_rows_in_seq_order` and `tailer_survives_a_transient_read_error`) —
because the tokio runtime is starved under parallel load and those margins disappear. A suite that
fails intermittently on the developer's own machine is not green, and it trains everyone to ignore
red.

Required shape:
- **Positive assertions** ("this event arrives"): poll until a GENEROUS deadline — e.g. loop with a
  total budget of ~5s, returning the moment the expected event appears. Fast in the common case,
  immune to load. Never `sleep(fixed)` then a tight `recv()` timeout.
- **Negative assertions** ("nothing is published during the outage"): a bounded wait is unavoidable,
  but it must be several multiples of `TAIL_INTERVAL`, never a value that only works on an idle
  machine.
- Verify by running **`cargo test -p services --lib`** — the FULL crate, which is the shape CI runs
  and where the contention actually occurs — at least **ten times consecutively**, pasting every
  result. All ten must pass. (Corrected 2026-08-12: the earlier bar named the SCOPED
  `--lib event_bus`. Attempt 3 passed that five times and was still failing ~1-in-3 under the full
  crate; the orchestrator accepted it on the weaker command. The scoped filter is not sufficient
  evidence.)
- Fixing a flake by LENGTHENING a deadline is not a fix. Attempt 3 raised the budget to 30s and the
  test still failed with `left: []` after exhausting it, which means the event never arrived at all —
  a synchronisation bug, not a slow machine. Find the race. Two known ones in this file: a second
  tailer is spawned with no readiness gap before rows are committed (every other spawn site in the
  file sleeps first), and `abort()` is not synchronous, so the first tailer may still be live on the
  same pool when the second starts.

## REQUIRED after attempt 6 (added 2026-08-12): VARIANT COVERAGE, and three more survivors

Attempt 6 closed the payload axis correctly and was rejected for the THIRD instance of the same
meta-defect: rigorous on the axes already attacked, structurally blind on a new one.

**SURVIVOR 1 (top severity) — every `NodeEvent` variant except `TaskCreated` can be silently dropped.**
Confirmed by the orchestrator in an isolated worktree: wrapping the publish in
`match &seq_ev.event { NodeEvent::TaskCreated { .. } => send, _ => {} }` while still advancing the
cursor passes the ENTIRE suite — `test result: ok. 263 passed; 0 failed`.
This is STRUCTURAL, not accidental: every commit helper in both files builds `NodeEvent::TaskCreated`,
and `delivered()` PANICS on any other variant, so the suite cannot express an expectation about eight
of the nine variants at all. **Blast radius is concrete and in this plan:** tasks 006/007/008 emit
`TaskStatusChanged`, `AttemptStarted`/`Finished`/`Failed` and node-runner activity events. Under this
mutant the bus keeps carrying `TaskCreated` — so it looks healthy — while every event phase 3 exists
to deliver is lost permanently, cursor advanced past it.

**SURVIVOR 2 — `seq` may be the batch POSITION rather than the journal's seq.** Publishing
`SequencedEvent { seq: last_published + i, .. }` survives (`263 passed; 0 failed`): positional and
real seqs are indistinguishable on a contiguous journal and no test ever produces a gapped one. Gaps
are NOT hypothetical — `seq` is `INTEGER PRIMARY KEY AUTOINCREMENT` precisely so deleted seqs are
never reused, and `event_journal::compact` stage 2 deletes oldest rows ignoring the cursor floor
(`queries.rs:141-161`), which does not protect the tailer because the tailer has no `trigger_cursors`
row. `seq` is what consumers persist as their cursor, so renumbering makes every downstream cursor
permanently wrong.

**SURVIVOR 3 — a per-pass publication cap loses the remainder of a batch.** Capping at 64 published
rows per pass while advancing the cursor survives (`263 passed; 0 failed`): no test commits more than
3 rows in one transaction. The sharpest trigger is the suite's OWN
`tailer_survives_a_transient_read_error` — a long outage produces a large catch-up batch on recovery,
and the cap discards exactly the rows the retry-without-advancing logic exists to preserve.

**SURVIVOR 4 — `EventBus::new` may ignore `broadcast_capacity`.** Replacing it with a hardcoded 1024
survives. Consequence: `lagged_refills_from_journal_and_resumes_live` passes whether or not `Lagged`
ever fires, so `subscribe_from`'s Lagged/refill arm has NO live coverage. **NOTE: this is a gap in
task 005, which is already marked passed.** `mod.rs` is in this task's file set, so fix it here rather
than reopening 005; record that in the ledger.

**Required tests** (the challenger's design, verified by it in both directions at `267 passed`):
1. `every_event_variant_is_published_with_its_body_intact` — commit one of ALL NINE variants in ONE
   transaction; compare `Vec<(seq, serde_json::Value)>` of delivered vs committed. Full-body JSON
   comparison is deliberate: `NodeEvent` has no `PartialEq`, and `RowId` cannot carry `exit_code`,
   `old_status`/`new_status`, `reason`, `executor` or `entity_count`. **Do NOT just add a
   `TaskDeleted` arm to `RowId`** — that leaves every other variant's payload fields unasserted.
2. `a_gap_in_the_journal_does_not_renumber_the_rows_after_it` — append 4 rows and `DELETE` seqs 1 and
   3 INSIDE THE SAME TRANSACTION (so no intermediate state is ever visible), then assert the
   survivors arrive as seqs 2 and 4.
3. `a_batch_larger_than_the_broadcast_buffer_is_published_whole` — 200 rows in one transaction, with
   `broadcast::channel(N*4)` so `RecvError::Lagged` cannot mask the result.
4. `new_honours_the_requested_broadcast_capacity` (`mod.rs`) — a capacity-2 bus must yield
   `TryRecvError::Lagged(1)` after three queued events.

**Mutation proofs (xi)-(xiv), both directions**, one per survivor above; each must fail its own test
and only its own. All TEN prior proofs must still behave as recorded.

A reference remedy exists at
`/data/Code/vk-swarm/.claude/worktrees/agent-a8261feb9ac00f4bb/task-013-attempt6-challenger-remedy.patch`.
**Read it, do not paste it** — verify every assertion yourself and re-run both directions; an
unverified patch is exactly the kind of borrowed confidence this task has been rejected for twice.

**Explicitly NOT findings** (do not "fix"): holding the last row of a batch for one pass (benign
latency wobble, no loss or reorder); dropping the `mark` upper bound; moving the readiness send after
the first pass; `last_published = mark` after the batch loop. All semantically equivalent.

## REQUIRED after attempt 5 (added 2026-08-12): ASSERT THE PAYLOAD, not just the seq

Attempt 5 closed the skip blind spot completely and was rejected for the SAME SHAPE of defect on a
different axis. Verified by the orchestrator in an isolated worktree: a tailer that keeps every
`seq` but replaces every published `event` with a fabricated
`NodeEvent::TaskCreated { task_id: Uuid::nil(), project_id: Uuid::nil() }` passes the ENTIRE suite —
`test result: ok. 263 passed; 0 failed`.

Every assertion in both test modules is on `ev.seq` alone. A count of payload assertions in the test
bodies is literally **0**; every `NodeEvent::` occurrence is a CONSTRUCTION site for a row being
committed, never an assertion on what was RECEIVED. `crates/db`'s `event_journal` tests do not cover
it either — they assert seqs and counts only. So nothing in this workstream checks that the event
body DELIVERED equals the body COMMITTED, which is the tailer's actual job. Seq was simply the axis
that was cheap to assert.

**Required.** Assert payload identity everywhere the tailer's output is checked. Commit rows with
DISTINCT identifying payloads (a fresh `task_id` per row, returned alongside the seq) and assert the
received event carries the expected one. `NodeEvent` has no `PartialEq`, so destructure the variant —
no `crates/db` change is needed and this task's file set is unaffected.

**A single-row assertion is NOT sufficient, and this is the trap.** The challenger built that remedy,
tested it, and found it incomplete: with payload identity added to `assert_publishes_exactly` (which
commits ONE row) and to the readiness test, the fabricated-payload mutation goes red (257/6) but
reversing payloads WITHIN a batch while preserving seqs still passes `263 passed; 0 failed`. Payload
identity must be asserted at EVERY site that collects the tailer's output into a `Vec`:
- `tailer_publishes_committed_rows_in_seq_order` (the `vec![1, 2, 3]` batch)
- `tailer_does_not_republish_across_passes` (its `first_pass` collection)
- `tailer_resumes_from_its_high_water_on_restart` (the `vec![5, 6]` collection)
- plus `assert_publishes_exactly` and `a_row_committed_after_readiness_is_never_dropped`

**Two mutation proofs, both directions, both must go RED:**
- **(ix) fabricated payload** — publish `NodeEvent::TaskCreated { task_id: Uuid::nil(), project_id:
  Uuid::nil() }` while keeping `seq_ev.seq`.
- **(x) batch payload permutation** — keep every seq, reverse the payloads within the batch. This is
  the one that survives a single-row-only fix.
All EIGHT existing proofs must still pass in both directions.

**Also fix (cosmetic, one line):** the doc comment at `tailer.rs:84` ends on the dangling fragment
"… Two problems." — editing residue.

**Recorded, NOT required (non-blocking, see the ledger).** Both fault-injection tests establish "the
fault fired" via fixed 225ms silence windows, and that silence is equally satisfied by a tailer that
is not polling at all — with `TAIL_INTERVAL` at 2000ms plus cursor-advance-on-error, both fault tests
PASS while carrying real data loss. Not a live defect at the shipped 75ms. Do not fix it in this
attempt; do not make it worse by lengthening `zero_receivers_does_not_stall_the_cursor`'s 300ms gap
or slowing the poll, because that gap is the only thing pinning `TAIL_INTERVAL` small.

## REQUIRED after attempt 4 (added 2026-08-12): an OBSERVABLE readiness signal

Attempt 4 passed Stage 1, ten consecutive full-crate runs, and five mutation proofs — and still let
a real **at-least-once delivery violation** through. Verified by the orchestrator in an isolated
worktree at `de75b78f`: a tailer that silently DROPS the first row it would ever publish, while
still advancing its cursor past it (so the row is lost forever), passes the ENTIRE suite —
`test result: ok. 262 passed; 0 failed`.

**Why it escapes.** `probe_until_live()` commits probe rows until one comes back, then makes every
downstream assertion RELATIVE to that row (`base + 1`, `base + 2`, …). Drop the first row and the
probe simply retries, the SECOND row becomes `base`, and every relative assertion still holds. The
declared deviation that replaced the dictated absolute seqs (`vec![1,2,3]`) with relative ones was
justified in the ledger as "strength INCREASED". It did strengthen history-replay detection via the
`floor` guard, but it opened a blind spot on the tailer's core invariant — the one that makes
journal-first worth doing at all.

**Why a relative assertion can never close this.** On startup a row committed BEFORE the tailer's
initial `high_water_mark` read is CORRECTLY not published (the tailer starts at the current mark by
design). Without an observable readiness signal a test cannot distinguish that legitimate skip from
a dropped row — which is exactly why the probe has to retry, and exactly why the retry hides the
bug. The ambiguity is structural; no amount of test cleverness removes it.

**The fix is a real contract, not a test trick.** `spawn` must signal once its initial
`high_water_mark` has resolved:

```rust
pub fn spawn(pool: SqlitePool, sender: broadcast::Sender<SequencedEvent>)
    -> (JoinHandle<()>, tokio::sync::oneshot::Receiver<()>)
```

Signal AFTER `last_published` has been ASSIGNED (not between the read and the assignment) and
BEFORE the first poll pass; use `let _ = ready_tx.send(())` so a dropped receiver cannot panic the
tailer.

**Do not misread what readiness buys you (challenger `panel-013-a4-race`, 2026-08-12).** A readiness
signal proves the initial READ COMPLETED. It does NOT prove the cursor EQUALS the mark: a mutant can
signal readiness and then set `last_published = mark + 1`, satisfying the happens-before edge while
the skip stays invisible. **It is the ABSOLUTE `seq == 1` assertion that kills mutations (vi) and
(vii); readiness is what makes that assertion SOUND rather than flaky.** They fix two different
problems and you need both. When you run the mutation proofs, the target that must fail is the
absolute assertion — do not score the proof against the readiness signal.

**Readiness also does NOT mean "the tailer has processed everything committed so far".**
`zero_receivers_does_not_stall_the_cursor` still needs the tailer to have PASSED OVER the three
zero-receiver rows before the test subscribes, which it currently buys with a fixed 300ms gap
(`tailer.rs:568`). Only observing a publication can prove that, and the probe-receiver drop already
does. Leave that mechanism alone; readiness does not replace it. (It is not a finding today: if the
gap is exceeded the assertion flips from `base+4` to `base+1` and FAILS rather than passing
vacuously — 16 full-suite runs including 6-way concurrent contention never tripped it.) `EventBus::new` may drop
the receiver, but record in the ledger that task 014 will likely want it surfaced — 014 must prove
the tailer is connected on a real deployment and faces this identical race.

**Then:**
1. **NEW test `a_row_committed_after_readiness_is_never_dropped`** — fresh pool, `spawn`, AWAIT
   readiness, assert `high_water_mark == 0`, subscribe, commit ONE row, assert the received event
   has **`seq == 1`, an ABSOLUTE assertion**. This is the test the drop-first mutation must fail.
   **Do NOT ship the one-line variant of this fix.** Challenger `panel-013-a4-race` validated
   `assert_eq!(base, 1, "fresh journal: ...")` bolted onto the existing probe: it does kill both skip
   mutants (`left: 2, right: 1`) and passed once on clean code. But WITHOUT the readiness signal it is
   flaky BY CONSTRUCTION, and flaky in the one direction that matters — if the tailer's initial mark
   read legitimately lands after the probe's first commit, the tailer correctly starts at 1, the probe
   correctly rebases to `base = 2`, and the assertion fails on CORRECT code. That is the identical
   spawn-vs-commit race that failed ~3-in-8 on attempt 3, re-entering through the assertion instead of
   through the deadline. Readiness is what makes the absolute assertion sound rather than lucky; ship
   them together or not at all.
2. **DELETE `probe_until_live` and await readiness instead**, restoring the absolute seq assertions
   this task dictated originally (`vec![1, 2, 3]`). Readiness is a happens-before edge that costs no
   journal rows, so the deviation becomes unnecessary rather than merely tolerated. Keep the `floor`
   history-replay check as an explicit assertion wherever it currently adds value — it was a genuine
   improvement and must not be lost in the revert.
3. **`zero_receivers_does_not_stall_the_cursor` keeps its probe-receiver drop** — that fix was
   correct and independent of this one; readiness only makes its setup deterministic too.

   **Where to AWAIT readiness in the outage tests.** In
   `tailer_retries_the_initial_high_water_mark_...` the signal necessarily arrives LATE — the initial
   retry loop only breaks after the rename-back — so await readiness AFTER the repair, not after the
   spawn. Same ordering applies to the restart test. Awaiting before the repair would hang until the
   test's own deadline and read as a failure of the tailer rather than of the test.
4. **Pin the two non-fresh-journal probes to an EXACT value.** `tailer_resumes_from_its_high_water_on_restart`
   and `tailer_retries_the_initial_high_water_mark_...` currently assert `base > 3`. That is the same
   one-sided guard that lets the skip through — under the `mark + 1` mutation `base` becomes 5 and
   `> 3` still passes. Assert the exact expected seq (`base == 4`) once readiness makes it
   deterministic.

**SANCTIONED FALLBACK if the `spawn` signature change is genuinely blocked** (e.g. it collides with
a caller you may not edit — task 005's `EventBus::new` is in your file set, so this should NOT
happen; if it does, STOP and report rather than improvising). Challenger `panel-013-a4-mutation`
proposed, and `panel-013-a4-race` assessed, an alternative ordering that needs NO production change:
spawn, then commit NOTHING for a generous bounded period (seconds — many multiples of
`TAIL_INTERVAL`), THEN commit the first row and require it to arrive. It is sound and it does kill
the skip mutant (on a fresh journal the mutant's cursor is 1, so the first row never arrives and the
positive assertion times out). It is nonetheless the FALLBACK, not the primary: it converts a
structural race into a bounded-probability tail rather than eliminating it, whereas the readiness
signal orders the initial read before the first commit deterministically. Prefer readiness; use this
only if blocked, and record the reason in the ledger.

**Mutation proofs required (all seven).** The five from attempt 4 must still fail their targets, PLUS
two skip-direction mutations, both confirmed to SURVIVE the attempt-4 suite:
- **(vi) drop the first row ever published while advancing the cursor** (orchestrator, reproduced in
  an isolated worktree at `de75b78f`, `262 passed; 0 failed`):
  ```rust
  if !dropped_first { dropped_first = true; last_published = seq_ev.seq; continue; }
  ```
- **(vii) start the cursor one row too high** (challenger `panel-013-a4-race`, `262 passed; 0 failed`,
  11.34s vs a ~6s baseline — the burned probe timeouts are the mutant's own fingerprint). In `spawn`'s
  initial retry loop: `Ok(mark) => break mark,` becomes `Ok(mark) => break mark + 1,`. Since
  `read_range` is `WHERE seq > ? AND seq <= ?`, this skips exactly the first row committed after
  startup, on every start AND every restart. This is the better mutation of the two: it is a
  one-character off-by-one in PRODUCTION code, the shape a real bug actually takes.

Both must FAIL after the fix. Note the REPLAY direction (`break mark - 1`) is already caught today by
`tailer_resumes_from_its_high_water_on_restart` — it is only the SKIP direction that is unbound, and
that asymmetry is the whole finding.

Prove every mutation in an ISOLATED worktree (`git worktree add`, own `CARGO_TARGET_DIR`) — two
challengers mutating one shared tree corrupted each other's evidence on this task and it must not
recur.

## Change
**File:** `crates/services/src/services/event_bus/tailer.rs`
**Anchor:** new file inside the `event_bus/` directory module task 005 already created. This task
does NOT restructure anything — 005 authored `event_bus/mod.rs` as a directory module from the start
precisely so this task only adds a sibling file.

**After:** the tailer — the component that makes "journal-first, broadcast-second" structural (D10).

```text
// Initial mark: RETRY, never fall back to 0 (property 1 binds this path).
// Log once or back off here -- at TAIL_INTERVAL=75ms an unbounded warn! is ~13 lines/second forever.
let mut last_published = loop {
    match high_water_mark(pool).await {
        Ok(mark) => break mark,
        Err(e) => { warn_once!(error = ?e, "high-water mark unavailable; retrying");
                    sleep(TAIL_INTERVAL).await; }
    }
};

loop {
    match high_water_mark(pool).await {
        Ok(mark) => match read_range(pool, last_published, mark).await {
            Ok(rows) => for ev in rows { let _ = sender.send(ev.clone()); last_published = ev.seq }
            Err(e)   => warn!(error = ?e, "tail read failed; retrying"),      // do NOT advance
        },
        Err(e) => warn!(error = ?e, "high-water mark failed; retrying"),      // do NOT advance
    }
    sleep(TAIL_INTERVAL).await
}
```

Four properties to preserve exactly:

1. **Start at the current high-water mark, not 0.** The tailer feeds the LIVE channel. Historical
   replay belongs to `subscribe_from`, which reads the journal directly. A tailer starting at 0
   would flood every new subscriber's live channel with history and force an immediate `Lagged`.
   **This binds the ERROR path too (2026-08-12).** Attempt 1 wrote
   `Err(e) => { warn!("starting from 0"); 0 }` — silently doing the one thing this property forbids,
   on a path that is genuinely reachable (a pool still warming, brief disk contention at process
   start). Do NOT fall back to 0. Retry until a mark is obtainable — log and sleep `TAIL_INTERVAL`,
   then try again — so the tailer simply begins publishing a little later instead of flooding every
   subscriber with the entire history.
2. **`send` errors are ignored deliberately.** `broadcast::Sender::send` returns `Err` when there
   are zero receivers, which is the normal idle state of a node with no subscribers. That is not a
   failure. Advance `last_published` regardless — the journal, not the channel, is the authority.
3. **A read error must not end the loop.** Log and retry on the next tick. Do NOT advance
   `last_published` on a failed read.
4. **One tailer per DBService**, owned by `EventBus` and spawned at startup (task 014). Two tailers
   on one channel would double-publish; consumers tolerate duplicates, but it is pure waste.

`TAIL_INTERVAL` is a named `const` with a comment. It is the publication-latency knob: the spec
accepts tail-interval-bounded latency because every named consumer (P6 triggers, P7 MCP
observability, the SSE endpoint) is non-interactive. Something in the 50-100ms range is appropriate;
justify the value in the ledger.

**File:** `crates/services/src/services/event_bus/mod.rs`
**Anchor:** the existing module created by task 005.
**Change:** add `mod tailer;` and a constructor/handle so `EventBus` owns the spawned tailer task.
Retain a `JoinHandle` (or an abort handle) so shutdown can stop it cleanly rather than leaking it.

**Retaining the handle is NOT enough — expose a way to USE it (2026-08-12).** Attempt 1 stored the
handle in a private `Arc<Mutex<Option<JoinHandle>>>` that nothing ever read: no `.abort()`, no
accessor, three references in the whole file (declaration, `Clone`, construction). Dropping a tokio
`JoinHandle` DETACHES the task, it does not cancel it — proven empirically by a challenger, which
dropped the `EventBus` entirely and watched the tailer keep publishing. That is the exact leak this
property forbids, and it also makes task 014's REQUIRED test `shutdown_stops_the_background_tasks`
(014 failing-test 5) unsatisfiable, because 014's file set is `crates/local-deployment/src/lib.rs`
alone and cannot reach into this module.

Add `pub async fn shutdown(&self)` on `EventBus`: lock the handle, `take()` the `Option`, and
`.abort()` it. `take()` makes repeat calls no-ops. **It does NOT make clones independent:** every
clone shares one `Arc`, so one clone's `shutdown()` stops the tailer for ALL of them, and any other
clone's `subscribe_from` stream will then replay the journal and park in `Live` forever with no error.
That is the intended consequence of property 4 (one tailer per `DBService`) — document it on the
method so only the owning deployment calls it. Note also that `abort()` is NOT synchronous: it cancels
at the task's next await point.

Add a test that constructs an `EventBus`, calls `shutdown()`, **waits until the tailer has actually
stopped**, then commits a journal row and asserts NOTHING is published. Also fix `new()`'s doc
comment, which currently claims the tailer stops when the `EventBus` is dropped — that claim is false.

**Task 014 note.** Because `shutdown()` `take()`s and drops the handle, nothing outside this module
can call `is_finished()`. Task 014's failing-test 5 (`shutdown_stops_the_background_tasks`, worded
"assert the spawned tasks terminate") is therefore satisfiable ONLY behaviourally: call
`deployment.event_bus().shutdown().await`, then commit a row and assert nothing is published.
`LocalDeployment::new` is already `async` (`crates/local-deployment/src/lib.rs:156,165`) and already
spawns background work at `:171`, so the `.await` is reachable from where 014 wires startup.

## Allowed moves
ONLY the new `tailer.rs` and the `mod tailer;` + handle additions in the existing
`event_bus/mod.rs`. Do NOT change `subscribe_from`'s algorithm (task 005 owns it). Do NOT spawn the
tailer from deployment startup here — task 014 owns all startup wiring. Do NOT add a sender to
`crates/db`.

## STOP triggers
- You cannot obtain a `high_water_mark` without a pool — the tailer must hold one; if `EventBus`
  does not already carry it, STOP rather than reaching for a global.
- The obvious implementation requires `crates/db` to know about the sender — STOP. That inversion is
  precisely the design the adversarial review disproved.
- `crates/services/src/services/event_bus/mod.rs` does not exist when this task starts — task 005 did
  not run or authored a flat `event_bus.rs` instead; STOP rather than restructuring it here, which
  would write to a path outside this task's declared file set.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services event_bus"

Record in the ledger: the chosen `TAIL_INTERVAL` and why, and confirmation that the tailer starts at
the high-water mark rather than 0.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 013` exits 0

## REQUIRED after attempt 7 — panel 5: three "does not give up" assertions are DEFEATED, and nothing pins ONE tailer

Attempt 7 passed Stage 1, ten consecutive full-crate runs, and four survivor mutations. The fifth
adversarial panel then found two axes the suite is structurally unable to state a claim about. This
is the fifth panel in a row to find a real defect on a NEW axis; treat the list of closed axes as
the map of where NOT to look, not as evidence the suite is complete.

Nothing in the earlier REQUIRED sections is retired by this one — absolute seq, payload bodies,
variant coverage, gaps, batch-vs-buffer and broadcast capacity all still stand exactly as written.
These two items are additive.

### Item 1 — the retry loops have no assertion about retry DURATION, on either arm

Three assertions exist whose stated purpose is "the tailer does not give up":

- `tailer.rs:704` — `"tailer should survive the transient read error"`
- `tailer.rs:904` — `"tailer should continue running after a read error"`
- `tailer.rs:1276` — `"tailer must retry the initial high-water mark, not give up"`

All three are **timing-bound rather than property-bound**. The main-loop pair fire ~450ms into the
outage (a 225ms sleep plus a 225ms silence `timeout`), which at the shipped `TAIL_INTERVAL = 75ms` is
about six poll attempts. The initial-loop test holds its outage 750ms, which at the 100/200/400/800…
backoff is about four retries. The panel proved both arms with count-guarded mutations:

- main loop, `return` after 10 consecutive failures (~750ms of outage): **`267 passed; 0 failed`**,
  twice. At N=1 the same mutation kills both tests, proving they ARE wired to the branch — so this is
  a defeated assertion, not missing coverage.
- initial loop, `break 0` after 10 retries (~6.3s): **`267 passed; 0 failed`**, three times. At N=1 it
  kills `tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero` — the test whose
  entire reason to exist is forbidding fall-back-to-0.

Blast radius, and why this is not the already-recorded non-blocking latency item: the tailer is the
ONLY producer of live events. If it returns, the broadcast channel goes silent with no error and no
restart, and `subscribe_from`'s Live arm parks on `rx.recv().await` forever because `EventBus` still
holds the sender. A sub-second DB blip — a WAL checkpoint, a pre-migration backup, a second instance
on the same file — would permanently kill phase 4/5 SSE delivery for the process lifetime while every
health surface still reads green. The initial-loop arm is attempt 1's fall-back-to-0 bug merely
delayed: a startup outage past the budget replays the ENTIRE journal onto the live channel.

**Fix the three assertions IN PLACE. Do not add shadow tests alongside them** — the assertion that is
defeated is the one that must be made to bite, and a new test next to a still-toothless old one
leaves the old one lying about what it proves.

- `tailer_survives_a_transient_read_error`: extend the outage held before the `!is_finished()`
  assertion from 225ms to **at least 1500ms** (≥20 poll attempts). Keep the 225ms silence `timeout`
  as-is — it serves a different purpose (proving the fault fired).
- `a_failed_read_does_not_end_the_loop_or_advance_the_cursor`: same, **at least 1500ms**.
- `tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero`: extend its outage from
  750ms to **at least 4000ms** (≥8 retries at that backoff).

**Update the comments that justify the old numbers.** `"Wait several multiples of TAIL_INTERVAL
(75ms * 3 = 225ms)"` sitting above a 1500ms sleep is precisely the stale-rationale drift this process
exists to catch.

`tokio::time` pause/auto-advance is ALLOWED instead of real sleeps if it proves the identical property
and the mutation proofs below still kill. It is not required — the tailer does real sqlx I/O, so
auto-advance may not behave. Real wall-clock is the safe path and its cost is accepted; the tests run
on parallel threads, so the suite grows by roughly the longest single window, not the sum.

**DECLARE THE RESIDUAL, do not hide it.** No finite wall-clock window can exclude an arbitrarily large
finite give-up budget — a threshold of 100 would still pass a 1500ms test. The windows above are
chosen to exceed any budget a plausible "add a retry limit" change would use. Record that limit
explicitly in the ledger. An overstated claim here is worse than the gap itself.

Required mutation proofs (count-guarded, `assert s.count(OLD) == 1`):

1. Main loop gives up after **10** consecutive failures → BOTH `tailer_survives_a_transient_read_error`
   and `a_failed_read_does_not_end_the_loop_or_advance_the_cursor` must FAIL.
2. Initial loop `break 0` after **10** retries →
   `tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero` must FAIL.

### Item 2 — nothing asserts that `EventBus::new` spawns exactly ONE tailer

The panel added a second `tokio::spawn` of the tailer inside `EventBus::new` on the same channel —
preserving `Clone` and aborting both handles in `shutdown()`, so shutdown semantics stay intact — and
the suite returned **`267 passed; 0 failed`** three times. Its scratch probe proved the mutation is a
genuine defect and not dead code:

```text
PANEL PROBE: the bus delivered the single committed row at seq 1 exactly 2 time(s)
PANEL PROBE CONTROL: the bus delivered the single committed row at seq 1 exactly 1 time(s)
```

No test can catch it today: every `mod.rs` test except `shutdown_stops_the_tailer` drives `sender` by
hand with fabricated `SequencedEvent`s and is deliberately tailer-independent, and all twelve
`tailer.rs` tests call `tailer::spawn` directly and never touch `EventBus::new`. So the suite nowhere
states "the bus publishes each journal row exactly once."

Add ONE test in `mod.rs` that:

- drives **`EventBus::new`** — not `tailer::spawn`; that is the whole point
- subscribes via `bus.sender().subscribe()` (the API `mod.rs:110` names as the tailer-side consumer,
  and the one that sees true duplicates, since `subscribe_from`'s Live arm dedups on `ev.seq > last`)
- commits ONE row, waits for its delivery, then waits a bounded further window (several
  `TAIL_INTERVAL`s) and asserts **no second copy arrives**

Required mutation proof: `EventBus::new` spawns a second tailer on the same channel, with `shutdown`
still aborting both → the new test must FAIL, and `shutdown_stops_the_tailer` must NOT be the test
that fails.

Blast radius if left open: 2× journal polling per bus against the same SQLite pool, halved effective
broadcast buffer (pushing `subscribe_from` into its `Lagged`/refill path far sooner, two extra queries
each time), and true duplicates for any direct `sender().subscribe()` consumer — which task 014's
startup wiring is the natural place to get wrong.

### Axes the panel checked and CLEARED — do not re-attack these

- **Replay-to-live handoff.** Moving the `subscribe()` after the journal read in `subscribe_from`
  reliably kills `no_journaled_event_is_skipped_across_the_handoff` (3/3). Genuinely covered.
- **Ordering under concurrent writers.** Unreachable on SQLite: writers serialise, so seq order equals
  commit order and no gap-below-cursor can form. Panel probe: `A got seq 1; B BLOCKED (timed out)
  until A committed`. Recorded so no future round burns time here.
