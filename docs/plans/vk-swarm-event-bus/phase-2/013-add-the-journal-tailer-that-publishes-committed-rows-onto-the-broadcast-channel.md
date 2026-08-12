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
5. `tailer_survives_a_transient_read_error` — make one read fail, then succeed; assert the tailer
   logs and continues rather than terminating. A tailer that dies on one error silently stops the
   entire bus. **`pool.close()` is NOT an acceptable way to do this (2026-08-12): it is irreversible
   in sqlx, so there is no "then succeed" half and attempt 1's version proved only non-termination.**
   Induce a genuinely transient failure instead — e.g. `chmod 000` the SQLite file so reads fail,
   then restore the mode — and assert BOTH that the tailer is still running AND that a row committed
   during the outage is published afterwards. That second assertion is what proves property 3's
   "do NOT advance `last_published` on a failed read": if the cursor had advanced past the outage,
   that row would be lost forever.
6. `zero_receivers_does_not_stall_the_cursor` — NEW (2026-08-12). Property 2 says `send` errors are
   ignored deliberately and `last_published` advances regardless, because zero receivers is the
   normal idle state of a node. Every attempt-1 test subscribed BEFORE publishing, so `send` never
   returned `Err` and this property was never exercised — a mutation that only advanced on `send`
   success survived the whole suite. Required shape: commit rows with NO subscriber attached, let a
   pass run, THEN subscribe and commit one more row; assert only the NEW row arrives. If the cursor
   had stalled on the zero-receiver error, the earlier rows would be republished.
7. `a_failed_read_does_not_end_the_loop_or_advance_the_cursor` — NEW (2026-08-12). Property 3 has two
   halves and attempt 1 tested neither for the `read_range` call: because `read_range` is nested
   inside the `Ok(mark)` arm of the `high_water_mark` match, a closed pool trips the OUTER branch and
   `read_range`'s error arm never executes. Mutations that returned from the loop on a read error, and
   that advanced `last_published` by 1000 on a failed read, BOTH survived the suite. Structure the
   pass so a read failure is reachable and assert both halves: the task is still running afterwards,
   and no row is skipped.

## Change
**File:** `crates/services/src/services/event_bus/tailer.rs`
**Anchor:** new file inside the `event_bus/` directory module task 005 already created. This task
does NOT restructure anything — 005 authored `event_bus/mod.rs` as a directory module from the start
precisely so this task only adds a sibling file.

**After:** the tailer — the component that makes "journal-first, broadcast-second" structural (D10).

```text
last_published = high_water_mark(pool)?   // start live; replay is subscribe_from's job, not ours
loop {
    match read_range(pool, last_published, high_water_mark(pool)?) {
        Ok(rows) => for ev in rows { let _ = sender.send(ev.clone()); last_published = ev.seq }
        Err(e)   => tracing::warn!(error = ?e, "event journal tail read failed; retrying"),
    }
    tokio::time::sleep(TAIL_INTERVAL).await
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
`.abort()` it. Taking makes it idempotent, so repeat calls and clones are safe. Add a test that
constructs an `EventBus`, calls `shutdown()`, then commits a journal row and asserts NOTHING is
published. Also fix `new()`'s doc comment, which currently claims the tailer stops when the
`EventBus` is dropped — that claim is false.

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
