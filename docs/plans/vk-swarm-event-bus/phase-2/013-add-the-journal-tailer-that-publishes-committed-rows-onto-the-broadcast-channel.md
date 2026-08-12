---
id: "013"
phase: 2
title: "Add the journal tailer that publishes committed rows onto the broadcast channel"
status: rejected
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
- Verify by running `cargo test -p services --lib event_bus` at least **five times consecutively**
  and pasting every result. All five must pass.

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
