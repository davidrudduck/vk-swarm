//! Journal tailer that publishes committed rows onto the broadcast channel.
//!
//! The tailer implements "journal-first, broadcast-second" publication:
//! - Reads the journal periodically (tail interval bounded)
//! - Publishes newly-committed rows to the broadcast channel
//! - Consumers subscribe to the broadcast live, plus replay-to-live via subscribe_from
//! - The journal is the source of truth; broadcast is an optimization

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use db::models::event::SequencedEvent;
use db::models::event_journal;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// The interval at which the tailer polls the journal for new events.
///
/// 75ms is in the 50-100ms range recommended by the spec for tail-interval-bounded latency.
/// This balances:
/// - Responsiveness: 75ms mean latency for new events from commit to subscriber delivery
/// - Efficiency: a 75ms poll interval uses negligible CPU compared to busier production systems
/// - Subscribers: the broadcast buffer (64 events) at ~1-2 events/task spans up to ~32 typical tasks;
///   at 75ms polling, even a 10 task/sec rate (very high) only sees ~1 event per poll, so Lagged
///   refills are rare and subscribers stay nearly synchronized
const TAIL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(75);

/// Observable liveness counters for the tailer, shared with `EventBus` so a caller can tell
/// whether the tailer is alive WITHOUT inferring it from timing side-effects on the broadcast
/// channel.
///
/// This is the product gap named in the decisions-ledger: today, if the tailer dies, the channel
/// goes quiet, `subscribe_from` parks forever, and every health surface still reads green.
#[derive(Debug, Default)]
pub struct TailerHealth {
    /// Total number of poll passes attempted, whatever their outcome.
    pub polls_total: AtomicU64,
    /// Number of `Failed` outcomes since the last non-`Failed` outcome. Reset to 0 on `Idle` or
    /// `Published`.
    pub consecutive_failures: AtomicU64,
    /// The seq of the most recently published row. Unchanged by `Idle` or `Failed` passes.
    pub last_published_seq: AtomicI64,
}

/// The outcome of ONE poll pass. Deliberately has NO variant that ends the loop: "give up" is not
/// expressible here, which is the point of this type. See the decisions-ledger for task 016: the
/// prior approach detected instances of the poll loop terminating early one wall-clock window at a
/// time; this makes the defect unrepresentable instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PollOutcome {
    /// The journal had no rows above the cursor.
    Idle,
    /// `count` rows were read and published; the cursor advanced to the last of them.
    Published { count: usize },
    /// The high-water-mark read or the range read failed. The cursor is unchanged.
    Failed,
}

/// Runs exactly ONE poll pass: read the current high-water mark, read (and publish) any rows
/// above `cursor`, and report what happened. Contains no `sleep` and no loop — driving it directly
/// is synchronous and immune to machine load, which is what makes `PollOutcome`'s absence of a
/// terminating variant testable as a structural property rather than an inferred one.
///
/// Behaviour is unchanged from the loop body this was extracted from: same queries, same order,
/// same cursor-advance rule (advance regardless of broadcast send errors — the journal is the
/// authority), same `warn!`/`debug!` sites.
async fn poll_once(
    pool: &SqlitePool,
    sender: &broadcast::Sender<SequencedEvent>,
    cursor: &mut i64,
    health: &TailerHealth,
) -> PollOutcome {
    health.polls_total.fetch_add(1, Ordering::Relaxed);

    let outcome = match event_journal::high_water_mark(pool).await {
        Ok(mark) => {
            // Read all rows in (cursor, mark]
            // read_range returns Vec<SequencedEvent> with already-deserialized events
            match event_journal::read_range(pool, *cursor, mark).await {
                Ok(seq_events) => {
                    let count = seq_events.len();
                    for seq_ev in seq_events {
                        // Publish to the broadcast channel.
                        // Ignore send errors — they mean zero receivers (normal idle state).
                        // Advance the cursor regardless.
                        let _ = sender.send(seq_ev.clone());
                        *cursor = seq_ev.seq;
                    }
                    debug!(last_published = *cursor, "tailer pass completed");
                    if count == 0 {
                        PollOutcome::Idle
                    } else {
                        PollOutcome::Published { count }
                    }
                }
                Err(e) => {
                    // Read error: log and retry without advancing
                    warn!(error = ?e, "event journal tail read failed; retrying");
                    PollOutcome::Failed
                }
            }
        }
        Err(e) => {
            // Failed to fetch high-water mark: log and retry
            warn!(error = ?e, "failed to fetch high-water mark; retrying");
            PollOutcome::Failed
        }
    };

    match outcome {
        PollOutcome::Failed => {
            health.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        }
        PollOutcome::Idle => {
            health.consecutive_failures.store(0, Ordering::Relaxed);
        }
        PollOutcome::Published { .. } => {
            health.consecutive_failures.store(0, Ordering::Relaxed);
            health.last_published_seq.store(*cursor, Ordering::Relaxed);
        }
    }

    outcome
}

/// Spawns the journal tailer task.
///
/// The tailer runs a loop that:
/// 1. Reads from the journal starting just after the last-published seq
/// 2. Publishes each new event to the broadcast channel
/// 3. Advances the cursor regardless of broadcast send errors (the journal is the authority)
/// 4. Retries on read errors without advancing (no loss on transient failures)
/// 5. Sleeps until the next poll interval
///
/// Returns a `JoinHandle` to stop the tailer cleanly on shutdown, and a READINESS receiver that
/// resolves once the tailer's initial cursor is established and BEFORE its first poll pass.
///
/// # Readiness
///
/// `tokio::spawn` only SCHEDULES this task, and its initial `high_water_mark` read then has to
/// complete on sqlx's worker. Until that read has resolved, the tailer has no cursor, and a row
/// committed in the meantime is CORRECTLY not published (property 1: start at the mark, not 0).
/// That makes a legitimate skip observationally identical to a dropped row, so no caller — and no
/// test — can assert an absolute seq without an observable happens-before edge.
///
/// The readiness signal is that edge: once it resolves, the initial mark is fixed, so every row
/// committed afterwards is strictly above the cursor and MUST be published. The send is
/// `let _ = ...` deliberately — a caller that drops the receiver (as `EventBus::new` does) must
/// not panic the tailer.
pub fn spawn(
    pool: SqlitePool,
    sender: broadcast::Sender<SequencedEvent>,
    health: Arc<TailerHealth>,
) -> (JoinHandle<()>, oneshot::Receiver<()>) {
    let (ready_tx, ready_rx) = oneshot::channel();

    let handle = tokio::spawn(async move {
        // Start at the current high-water mark to replay only NEW events, not history
        let mut retry_count = 0u32;
        let mut last_published = loop {
            match event_journal::high_water_mark(&pool).await {
                Ok(mark) => break mark,
                Err(e) => {
                    // Log once, then silently retry with exponential backoff
                    if retry_count == 0 {
                        warn!(error = ?e, "failed to fetch initial high-water mark; retrying");
                    }
                    retry_count += 1;
                    let backoff_ms = std::cmp::min(1000, 50 * (1 << retry_count.min(4)));
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms as u64)).await;
                }
            }
        };

        // The counter starts where the CURSOR starts. Leaving it at its `Default` 0 while the
        // cursor sits at the high-water mark makes a busy journal read as "nothing has ever been
        // published" until the first post-start publish — which on a quiet node may never arrive.
        // That is the green-while-dead confusion these counters exist to remove, so it must not be
        // reintroduced by the initial value. Stored BEFORE readiness so a caller that observes
        // readiness observes a consistent cursor/counter pair.
        health
            .last_published_seq
            .store(last_published, Ordering::Relaxed);

        // Signal AFTER `last_published` has been ASSIGNED (not between the read and the
        // assignment, which would reopen the window in the exact place this finding lives) and
        // BEFORE the first poll pass (a signal sent after a pass would leave rows committed during
        // that pass in the same ambiguous window). The cursor is now fixed, so everything committed
        // from here on is above it and is owed to subscribers.
        //
        // NOTE what this does and does not buy: it proves the initial READ COMPLETED. It does NOT
        // prove the cursor EQUALS the mark — a tailer could signal here and then skip ahead, which
        // is exactly mutation (vii). Catching that is the job of the ABSOLUTE seq assertions in the
        // tests; readiness only makes those assertions sound rather than flaky.
        let _ = ready_tx.send(());

        // The driver's ONLY exit is the task being aborted. `poll_once`'s return type has no
        // variant that could end this loop — that is what makes the give-up defect class
        // unrepresentable rather than merely undetected.
        loop {
            let _ = poll_once(&pool, &sender, &mut last_published, &health).await;
            tokio::time::sleep(TAIL_INTERVAL).await;
        }
    });

    (handle, ready_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::event::NodeEvent;
    use db::test_utils::create_test_pool_with_migrations;
    use uuid::Uuid;

    /// Poll for an event with a generous deadline budget (~30 seconds).
    /// Returns the first event received before the deadline, or None if the deadline expires.
    /// Immune to runtime contention since it waits indefinitely (up to deadline) for the event.
    async fn poll_for_event(
        subscriber: &mut tokio::sync::broadcast::Receiver<SequencedEvent>,
        deadline: tokio::time::Instant,
    ) -> Option<SequencedEvent> {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, subscriber.recv()).await {
            Ok(Ok(ev)) => Some(ev),
            Ok(Err(_)) => None, // RecvError (channel closed)
            Err(_) => None,     // Timeout (deadline passed)
        }
    }

    /// The full identity of a committed journal row: its seq AND the payload fields that make it
    /// unique.
    ///
    /// Asserting `ev.seq` alone is what let attempt 5 through: a tailer that keeps every seq but
    /// replaces every published `event` with a fabricated
    /// `NodeEvent::TaskCreated { task_id: Uuid::nil(), project_id: Uuid::nil() }` passed the entire
    /// 263-test suite. Delivering the right body is the tailer's actual job, and seq was simply the
    /// axis that was cheap to assert.
    ///
    /// Every row these tests commit carries a FRESH `task_id`/`project_id` pair, so comparing
    /// `Vec<RowId>` pins WHICH committed row landed at WHICH seq — which a per-row check cannot do.
    /// That distinction matters: a mutation that reverses the payloads WITHIN a batch while
    /// preserving every seq survives a single-row payload assertion, and only whole-vector equality
    /// catches it.
    ///
    /// `NodeEvent` has no `PartialEq`, hence a local type that does rather than a `crates/db`
    /// change. `project_id` is carried alongside `task_id` at no cost, so a mutation that fabricates
    /// only one of the two fields has nowhere to hide either.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct RowId {
        seq: i64,
        task_id: Uuid,
        project_id: Uuid,
    }

    /// The identity of an event as DELIVERED by the tailer, for comparison against what was
    /// COMMITTED.
    ///
    /// Destructures rather than compares whole values because `NodeEvent` has no `PartialEq`. A
    /// non-`TaskCreated` body is itself a failure: every row these tests commit is a `TaskCreated`,
    /// so anything else means the tailer invented the payload.
    fn delivered(ev: &SequencedEvent) -> RowId {
        match &ev.event {
            NodeEvent::TaskCreated {
                task_id,
                project_id,
            } => RowId {
                seq: ev.seq,
                task_id: *task_id,
                project_id: *project_id,
            },
            other => panic!(
                "the tailer delivered {other:?} at seq {}; every row these tests commit is a \
                 TaskCreated, so any other body was fabricated rather than read from the journal",
                ev.seq
            ),
        }
    }

    /// The seqs of a row set, for the ABSOLUTE stale-expectation guards that sit in front of every
    /// payload assertion.
    ///
    /// Payload identity is asserted against what was committed, which is ground truth but says
    /// nothing about WHICH absolute seqs those rows took. The seq literals (`vec![1, 2, 3]`,
    /// `vec![5, 6]`, …) are what catch the skip mutations, and they stay hardcoded.
    fn seqs_of(rows: &[RowId]) -> Vec<i64> {
        rows.iter().map(|r| r.seq).collect()
    }

    /// Commits one journal row and returns its full identity (seq + payload).
    async fn commit_one(pool: &SqlitePool) -> RowId {
        commit_batch(pool, 1).await.remove(0)
    }

    /// Commits `n` journal rows in ONE transaction and returns their identities in commit order.
    ///
    /// One transaction matters: the rows become visible atomically, so a single tailer poll pass
    /// reads them as one batch. That is the only shape in which a within-batch payload permutation
    /// is observable at all.
    async fn commit_batch(pool: &SqlitePool, n: usize) -> Vec<RowId> {
        let mut tx = pool.begin().await.unwrap();
        let mut rows = Vec::with_capacity(n);
        for _ in 0..n {
            let task_id = Uuid::new_v4();
            let project_id = Uuid::new_v4();
            let event = NodeEvent::TaskCreated {
                task_id,
                project_id,
            };
            let seq = event_journal::append(&mut *tx, &event).await.unwrap();
            rows.push(RowId {
                seq,
                task_id,
                project_id,
            });
        }
        tx.commit().await.unwrap();
        rows
    }

    /// Blocks until the tailer signals readiness: its initial `high_water_mark` has resolved and
    /// no poll pass has run yet.
    ///
    /// This REPLACES attempt 4's `probe_until_live()`. That helper committed probe rows until one
    /// came back, then made every downstream assertion RELATIVE to it — which is why a tailer that
    /// silently DROPPED the first row it would ever publish passed the entire suite: the probe
    /// simply retried, the second row became the base, and every relative assertion still held.
    ///
    /// Readiness closes that hole at the source. It is a happens-before edge that costs no journal
    /// row, so a row committed after it is unconditionally owed to subscribers and its seq can be
    /// asserted ABSOLUTELY. The deadline is generous rather than tight, per this task's
    /// deadline-based waiting rule.
    async fn await_ready(ready: oneshot::Receiver<()>) {
        tokio::time::timeout(std::time::Duration::from_secs(30), ready)
            .await
            .expect("the tailer did not signal readiness within 30s")
            .expect("the tailer dropped its readiness sender without signalling");
    }

    /// The minimum number of CONSECUTIVE failed poll passes a held-open fault must produce before
    /// a driver-liveness test will accept that the driver survived it.
    ///
    /// This number is the detection floor for the give-up defect class: any driver that abandons
    /// the loop after fewer than this many consecutive failures stalls the counter below the
    /// target and fails. It is deliberately ABOVE the ~20 passes the deleted 1500ms windows
    /// covered, so attempt 2 is strictly stronger than the code it replaces rather than merely
    /// equivalent.
    const REQUIRED_CONSECUTIVE_FAILURES: u64 = 25;

    /// Waits until the tailer has recorded at least `target` CONSECUTIVE failed poll passes,
    /// returning the instant it arrives.
    ///
    /// This is the observable that replaces the two deleted 1500ms windows, and it is not the same
    /// kind of thing. A fixed wall-clock window asserts a claim about the CLOCK and then infers the
    /// driver from it: any give-up budget larger than the window passes, and every run on every
    /// machine burns the full window whether or not it was needed. Waiting on
    /// `consecutive_failures` asserts the claim directly — the driver must have executed `target`
    /// poll passes and stayed in the loop through all of them — and returns as soon as it has,
    /// rather than at a time chosen in advance.
    ///
    /// The deadline is a SAFETY NET, not the mechanism: it exists only so a stalled driver fails
    /// with a diagnosis instead of hanging the suite. It is generous (30s, matching `await_ready`
    /// and `poll_for_event`) precisely because it must never be the thing that decides the verdict.
    async fn await_consecutive_failures(health: &TailerHealth, target: u64) {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let seen = health.consecutive_failures.load(Ordering::Relaxed);
            if seen >= target {
                return;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the tailer recorded only {seen} consecutive failures in 30s while the fault was \
                 held open; at least {target} were required. A driver that gives up — returns, \
                 breaks, or otherwise leaves the loop — after fewer than {target} consecutive \
                 failures stalls this counter forever, which is exactly what this wait exists to \
                 catch and what a fixed wall-clock window could not."
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    /// The number of poll passes a driver-CADENCE test requires the tailer to complete before it
    /// will accept that the driver is still polling.
    ///
    /// This is the give-up detection floor for BOTH paths, and the number is load-bearing, so here
    /// is the derivation rather than the number alone.
    ///
    /// `consecutive_failures` moves only on `PollOutcome::Failed`, so `await_consecutive_failures`
    /// can only ever guard the FAILURE path. `polls_total` moves on EVERY pass whatever its
    /// outcome, so one wait on it covers idle, failure and adaptive backoff at once — a driver that
    /// abandons the loop on quiet, on error, or that slows to a crawl, all freeze or stall the same
    /// counter.
    ///
    /// A climb target kills a give-up budget B only if the target STRICTLY EXCEEDS B, because a
    /// give-up at B leaves the counter frozen at B, and a wait for anything at or below B is
    /// satisfied before the freeze is observable. The waits below start from a baseline the tests
    /// assert is under `MAX_CADENCE_BASELINE_POLLS`, so 50 clears the highest budget in this task's
    /// required mutation proofs (40) with margin, and clears it on the counter rather than on
    /// `is_finished()`.
    ///
    /// DECLARED RESIDUAL: a give-up budget of 50 or more is NOT caught by these tests. Detection
    /// cost by this mechanism is linear in the budget — 50 passes measures ~4.0s of wall clock at
    /// `TAIL_INTERVAL` — and timing is the only mechanism available here: virtual time is hazarded
    /// in the task file because this code does real sqlx file I/O on a blocking pool, and
    /// production changes beyond the counters are out of scope. The trade is stated rather than
    /// discovered later.
    const REQUIRED_POLL_CLIMB: u64 = 50;

    /// The largest `polls_total` a cadence test tolerates at the moment it starts its wait.
    ///
    /// A TRIPWIRE on the wait's starting count. It is deliberately NOT load-bearing for the kill,
    /// and saying so is the point of this comment.
    ///
    /// It is tempting to claim the kill depends on the baseline being near zero. It does not, and
    /// the arithmetic says why. Under a give-up at budget B the counter climbs to B and freezes, so
    /// any baseline read before the freeze satisfies `baseline <= B`. The wait's target is
    /// `baseline + REQUIRED_POLL_CLIMB`, so the kill condition `target > B` reduces to
    /// `REQUIRED_POLL_CLIMB > B - baseline`, which is hardest at `baseline = 0` and holds there for
    /// every B below 50. A LARGER baseline only makes the kill easier, and cannot cause a false red
    /// either: correct code needs 50 MORE passes (~4s) wherever it starts. So `REQUIRED_POLL_CLIMB`
    /// is sized against the worst case of 0 and the baseline is free.
    ///
    /// What this assertion actually buys is a diagnosis: if a future edit ever pre-advances
    /// `polls_total` from something other than this tailer's own driver — a shared counter, a
    /// pre-readiness warm-up loop — the cadence tests stop measuring what their names say, and this
    /// fails immediately with a number rather than degrading quietly. Cheap, and worth keeping for
    /// that alone.
    const MAX_CADENCE_BASELINE_POLLS: u64 = 5;

    /// Deadline for a `REQUIRED_POLL_CLIMB` climb, on a quiet journal and on a faulted one alike.
    ///
    /// 50 passes at `TAIL_INTERVAL` (75ms) is 3.75s of sleeping, plus a `high_water_mark` per pass
    /// against a tiny SQLite file. Both costs were MEASURED rather than assumed, 8 isolated runs
    /// each on a quiet 4-core development machine:
    ///
    /// | test | min | max | per pass |
    /// |---|---|---|---|
    /// | quiet journal | 3.99s | 4.02s | ~80ms |
    /// | journal renamed away (every pass fails) | 4.07s | 4.09s | ~81ms |
    ///
    /// A failing pass was expected to cost materially more — an errored `high_water_mark` plus a
    /// SQLite re-prepare on `SQLITE_SCHEMA` — and measurably does not (~1ms), which is why one
    /// deadline serves both rather than two tuned separately.
    ///
    /// Two independent things are bought at 20s:
    ///
    /// - LOAD MARGIN: 20s / ~4.1s = 4.9x. The test survives a machine nearly 5x slower than this
    ///   one before the deadline, rather than the driver, decides the verdict.
    /// - RATE FLOOR: 20s / 50 passes = 400ms per pass. Any adaptive backoff to 400ms or worse
    ///   fails here, and a driver that stops polling entirely fails at the deadline with a count.
    ///
    /// DECLARED RESIDUAL: a backoff FASTER than ~400ms per pass is not distinguishable from a
    /// loaded machine by timing alone and is not caught. That uncaught case is a latency
    /// regression; the give-up case, which is silent death, is caught cleanly because the counter
    /// freezes forever rather than merely slowing.
    const CADENCE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(20);

    /// Waits until the tailer has completed at least `climb` MORE poll passes than it had when the
    /// wait began.
    ///
    /// This is the observable that covers the whole give-up class, and it is strictly stronger than
    /// `await_consecutive_failures` in coverage: `polls_total` is incremented at the top of
    /// `poll_once` on every pass, so it advances on `Idle`, on `Published` and on `Failed` alike.
    /// A driver that returns after N quiet passes — panel 6's original finding, and the shape that
    /// survived attempt 2's failure-path-only guard — freezes this counter exactly as a driver that
    /// returns after N failures does.
    ///
    /// It also catches the shape where the loop never ends at all: an adaptive backoff to a long
    /// sleep keeps `!is_finished()` true forever and keeps every counter climbing, just far too
    /// slowly, so nothing that asks "is the task alive" can see it. Requiring a climb WITHIN a
    /// deadline asks about the RATE, which is what actually degraded.
    ///
    /// The deadline is a rate bound and a safety net, not the mechanism: the wait returns the
    /// instant the count arrives.
    async fn await_polls_climb(health: &TailerHealth, climb: u64, deadline: std::time::Duration) {
        let before = health.polls_total.load(Ordering::Relaxed);
        let target = before + climb;
        let expiry = tokio::time::Instant::now() + deadline;
        loop {
            let seen = health.polls_total.load(Ordering::Relaxed);
            if seen >= target {
                return;
            }
            assert!(
                tokio::time::Instant::now() < expiry,
                "the tailer completed only {seen} poll passes within {deadline:?}; it stood at \
                 {before} when this wait began and at least {target} were required. A driver that \
                 leaves its loop — on quiet, on error, or on anything else — freezes this counter \
                 forever, and a driver that backs off to a slower cadence stalls it past the \
                 deadline. Unlike consecutive_failures, polls_total advances on EVERY outcome, so \
                 this one wait covers the idle path, the failure path and adaptive backoff."
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    /// Asserts that NOTHING has been published, checked at the moment of the call rather than over
    /// a window.
    ///
    /// Paired with `await_consecutive_failures`, this covers the ENTIRE outage — every pass from
    /// the fault being induced to the counter arriving — instead of the leading 225ms of it. A
    /// `Lagged` is a FAILURE, not an empty channel: it means rows were published during the outage
    /// and then evicted from the buffer, which is precisely the loss this is checking for.
    fn assert_nothing_published(
        subscriber: &mut tokio::sync::broadcast::Receiver<SequencedEvent>,
        context: &str,
    ) {
        match subscriber.try_recv() {
            Ok(ev) => panic!("{context}; got seq {}", ev.seq),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // Expected: the journal was unreadable, so there was nothing to publish.
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => panic!(
                "{context}; the subscriber lagged by {n} events, so rows WERE published during the \
                 fault and then evicted from the broadcast buffer"
            ),
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                panic!("{context}; the broadcast channel closed unexpectedly")
            }
        }
    }

    /// `spawn` for tests that don't care about `TailerHealth` — everything except the health and
    /// driver-liveness tests, which need to hold onto the `Arc` they pass in and so call `spawn`
    /// directly. This just wires up a throwaway default so the signature change task 016 requires
    /// does not have to be repeated at every call site.
    fn spawn_ignoring_health(
        pool: SqlitePool,
        sender: broadcast::Sender<SequencedEvent>,
    ) -> (JoinHandle<()>, oneshot::Receiver<()>) {
        spawn(pool, sender, Arc::new(TailerHealth::default()))
    }

    /// Commits one row and asserts the tailer publishes exactly it — the same seq AND the same
    /// body — at exactly `expected_seq`.
    ///
    /// Both halves are ABSOLUTE. The first assertion pins what the journal assigned (so a surprise
    /// in seq allocation fails loudly instead of silently rebasing the test); the second pins what
    /// the tailer delivered. Called after `await_ready`, a correct tailer cannot skip this row, so
    /// there is no retry loop and nothing for a dropped row to hide behind.
    ///
    /// It also serves as a liveness proof: a publication is proof that the poll loop is running,
    /// which readiness alone does not establish (readiness fires before the first pass).
    ///
    /// The whole-`RowId` comparison is deliberately not decomposed into a seq check plus a payload
    /// check: the point is that the DELIVERED row is the COMMITTED row, and one equality states
    /// exactly that.
    async fn assert_publishes_exactly(
        pool: &SqlitePool,
        subscriber: &mut tokio::sync::broadcast::Receiver<SequencedEvent>,
        expected_seq: i64,
    ) {
        let committed = commit_one(pool).await;
        assert_eq!(
            committed.seq, expected_seq,
            "the journal assigned an unexpected seq; the test's absolute expectations are stale"
        );

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(subscriber, deadline).await {
            Some(ev) => assert_eq!(
                delivered(&ev),
                committed,
                "the tailer published {:?} where the row committed at seq {expected_seq} was owed; \
                 a row committed after readiness must be delivered, unskipped and unaltered",
                delivered(&ev)
            ),
            None => panic!(
                "the tailer published nothing within 30s; seq {expected_seq} was committed after \
                 readiness and must not be dropped"
            ),
        }
    }

    /// Commits one journal row directly to the RENAMED journal table.
    ///
    /// `event_journal::append` targets the original table name, so it cannot be used while the
    /// table is renamed away — and the initial-mark outage test needs rows committed DURING the
    /// outage, which is the only way to prove a correct tailer starts above them afterwards.
    async fn commit_one_to_hidden_journal(pool: &SqlitePool) -> RowId {
        let task_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let event = NodeEvent::TaskCreated {
            task_id,
            project_id,
        };
        let payload = serde_json::to_string(&event).unwrap();
        let seq = sqlx::query_scalar::<_, i64>(
            "INSERT INTO event_journal_hidden (event_type, payload) VALUES (?, ?) RETURNING seq",
        )
        .bind(event.event_type())
        .bind(payload)
        .fetch_one(pool)
        .await
        .unwrap();
        RowId {
            seq,
            task_id,
            project_id,
        }
    }

    /// The tailer's core invariant, asserted ABSOLUTELY: the FIRST row committed after the tailer
    /// is ready must be published, as seq 1, on a fresh journal.
    ///
    /// This is the test attempt 4 did not have, and its absence is why two SKIP mutations survived
    /// its whole 262-test suite — one of them a one-character off-by-one in production code
    /// (`break mark + 1`). Every assertion in that suite was relative to a probe row, so dropping
    /// the first row merely rebased the frame.
    ///
    /// It is sound only BECAUSE of the readiness signal. Without it, a tailer whose initial mark
    /// read lands after this commit correctly starts at 1 and correctly publishes nothing, and the
    /// assertion would fail on CORRECT code — the same spawn-vs-commit race that failed ~3-in-8 on
    /// attempt 3, re-entering through the assertion instead of the deadline.
    #[tokio::test]
    async fn a_row_committed_after_readiness_is_never_dropped() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // The cursor is fixed from here on, and no poll pass has run
        await_ready(ready).await;

        // The journal is empty, so the tailer's cursor is provably 0 and seq 1 is owed to us
        assert_eq!(
            event_journal::high_water_mark(&pool).await.unwrap(),
            0,
            "this test's absolute seq assertion requires a fresh journal"
        );

        let mut subscriber = tx.subscribe();

        let committed = commit_one(&pool).await;
        assert_eq!(
            committed.seq, 1,
            "the first row of a fresh journal must be seq 1"
        );

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let received = poll_for_event(&mut subscriber, deadline).await;

        tailer_handle.abort();

        match received {
            Some(ev) => assert_eq!(
                delivered(&ev),
                committed,
                "the first row committed after readiness must be published as seq 1 CARRYING THE \
                 BODY THAT WAS COMMITTED; a different seq means the tailer skipped it and advanced \
                 past it, a different body means the tailer did not deliver the journal's row"
            ),
            None => panic!(
                "the tailer published nothing within 30s; seq 1 was committed after readiness and \
                 cannot legitimately be skipped"
            ),
        }
    }

    #[tokio::test]
    async fn tailer_publishes_committed_rows_in_seq_order() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first (high-water mark is 0)
        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel BEFORE committing rows
        let mut subscriber = tx.subscribe();

        // Readiness fixes the cursor at 0 before anything is committed, so the seqs below are
        // ABSOLUTE — the shape this task dictated originally, restored now that the happens-before
        // edge exists. Attempt 4's relative `base + n` form is what let a dropped first row hide.
        await_ready(ready).await;

        // Now append and commit 3 rows, each carrying a DISTINCT payload so the delivered stream
        // can be matched row-for-row and not merely seq-for-seq
        let committed = commit_batch(&pool, 3).await;

        // Absolute, and hardcoded: `committed` is ground truth for WHICH rows exist but says
        // nothing about WHICH seqs they took, and the seq literals are what catch the skip
        // mutations
        assert_eq!(
            seqs_of(&committed),
            vec![1, 2, 3],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        // Collect the published events using deadline-based polling.
        // This is immune to runtime contention; waits up to deadline for events to arrive.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut received = vec![];
        for _ in 0..3 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => received.push(delivered(&ev)),
                None => break,
            }
        }

        tailer_handle.abort();

        // Whole-vector equality on (seq, payload), not on seqs alone. The three rows land in ONE
        // batch (one transaction, so one poll pass sees all three), which is the only place a
        // within-batch payload permutation is observable — and a permutation preserves every seq,
        // so a seq-only assertion or a per-row payload check both pass right through it.
        assert_eq!(
            received, committed,
            "tailer should publish the three committed rows, in seq order, each carrying the body \
             that was committed at that seq"
        );
    }

    #[tokio::test]
    async fn tailer_never_publishes_a_rolled_back_row() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Cursor fixed at 0 before anything is written, so the seqs below are absolute
        await_ready(ready).await;

        // Open a transaction, append an event, and roll back without committing.
        // Its payload is distinct from the committed row's below, so a tailer that somehow
        // delivered the rolled-back body under the committed row's seq would also be caught.
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            // Intentionally drop without committing
            drop(tx_write);
        }

        // Wait several multiples of TAIL_INTERVAL to ensure the tailer has polled multiple times
        // and confirmed no events were published (rolled back row is not committed).
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;

        // Verify no events were published during the rollback
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(_)) => panic!("tailer should not publish rolled-back rows"),
            _ => {
                // Expected: no event received
            }
        }

        // Now commit a different row. The rolled-back append released its seq (AUTOINCREMENT's
        // `sqlite_sequence` bump is rolled back with the transaction), so this row takes seq 1.
        let committed = commit_one(&pool).await;
        assert_eq!(
            committed.seq, 1,
            "a rolled-back append must release its seq, or this test's absolute expectation is stale"
        );

        // The silence window above is only attributable to the rollback once we know the tailer is
        // live at all — which this positive assertion supplies, after the fact rather than before.
        // It is the same evidence, in the only order the test's subject permits: there is nothing
        // to publish before the rollback without inventing a row that changes what seq 1 means.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                assert_eq!(
                    delivered(&ev),
                    committed,
                    "should only receive the committed row, body and all, not the rolled-back one"
                );
            }
            None => panic!("expected to receive the committed row"),
        }

        tailer_handle.abort();
    }

    #[tokio::test]
    async fn tailer_does_not_republish_across_passes() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Cursor fixed at 0 before anything is written, so the seqs below are absolute
        await_ready(ready).await;

        // Append and commit 3 journal rows, each with a distinct payload
        let committed = commit_batch(&pool, 3).await;
        assert_eq!(
            seqs_of(&committed),
            vec![1, 2, 3],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        // Collect the published events from the first pass using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut first_pass = vec![];
        for _ in 0..3 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => first_pass.push(delivered(&ev)),
                None => break,
            }
        }

        // The first pass MUST actually have published the rows — the right rows, with the right
        // bodies. Without this the whole test is vacuous: a tailer that publishes nothing at all
        // trivially "does not republish". Vector equality also covers the within-batch payload
        // permutation, which these three atomically-committed rows are read as.
        assert_eq!(
            first_pass, committed,
            "the first pass must publish the three committed rows, each carrying its own body"
        );

        // Wait for another poll interval (tailer should find no new events).
        // Use multiple multiples of TAIL_INTERVAL to ensure no republish occurs.
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;

        // Verify the second pass publishes nothing (bounded wait ensures we're checking real state)
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(_)) => panic!("tailer should not republish in the second pass"),
            _ => {
                // Expected: no new events
            }
        }

        tailer_handle.abort();
    }

    #[tokio::test]
    async fn tailer_resumes_from_its_high_water_on_restart() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Append and commit 3 rows. These predate the tailer and must never be published; their
        // identities are not needed, only their absence.
        assert_eq!(
            seqs_of(&commit_batch(&pool, 3).await),
            vec![1, 2, 3],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        // Create the broadcast channel and start the tailer
        // It will start at high-water mark (3) and won't republish the initial 3 rows
        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // Subscribe BEFORE readiness resolves, so a tailer that replays history cannot do it in a
        // window where nothing is listening
        let mut subscriber = tx.subscribe();

        // Readiness replaces the 10ms "give it a moment" gap: the mark is now provably resolved,
        // so the silence below is attributable to property 1 and not to a tailer that has not
        // started reading yet
        await_ready(ready).await;

        // Wait and verify no events were published yet (tailer started at the high-water mark).
        // Use multiple multiples of TAIL_INTERVAL to ensure the tailer has polled and found nothing.
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(_)) => panic!("tailer should not replay old rows"),
            _ => {
                // Expected: no events
            }
        }

        // Drop the tailer. `abort()` is NOT synchronous — it only cancels at the task's next await
        // point — so join the handle to get a real barrier. Without it the old tailer can still be
        // polling the same 5-connection pool while the new one performs its initial read.
        tailer_handle.abort();
        let _ = tailer_handle.await;

        // Create a new tailer (will start at the current high-water mark of 3)
        let (tx2, _rx2) = broadcast::channel(64);
        let (tailer_handle2, ready2) = spawn_ignoring_health(pool.clone(), tx2.clone());

        // Subscribe to the new tailer, again before readiness resolves
        let mut subscriber2 = tx2.subscribe();

        // The race that made this test fail with `left: []`: the rows used to be committed
        // immediately after `tokio::spawn`, so the tailer's initial high-water-mark read could
        // resolve after them, start at 5, and correctly never publish 4 and 5. Readiness makes
        // that impossible rather than unlikely.
        await_ready(ready2).await;

        // Pin the resumed cursor to an EXACT value. `base > 3` was a one-sided guard: under the
        // `break mark + 1` skip mutation the first published row becomes 5 and `> 3` still passes.
        // Seq 4 is the only row a correct restarted tailer may publish first — it neither replays
        // 1-3 nor skips ahead.
        assert_publishes_exactly(&pool, &mut subscriber2, 4).await;

        // Now append 2 more rows, in one transaction so they arrive as a single batch
        let committed = commit_batch(&pool, 2).await;

        // Absolute: seqs 1-3 predate the restart and seq 4 was the cursor pin above
        assert_eq!(
            seqs_of(&committed),
            vec![5, 6],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        // Collect the published events using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut received = vec![];
        for _ in 0..2 {
            match poll_for_event(&mut subscriber2, deadline).await {
                Some(ev) => received.push(delivered(&ev)),
                None => break,
            }
        }

        tailer_handle2.abort();

        // Only the 2 new rows should be published, in order, each with the body committed at its
        // seq, and with no history mixed in
        assert_eq!(
            received, committed,
            "new tailer should resume from high-water and publish only the new rows, bodies intact"
        );
    }

    /// The tailer must survive an OUTER-arm failure (`high_water_mark` unreadable) for at least
    /// `REQUIRED_CONSECUTIVE_FAILURES` consecutive passes, then recover without having advanced its
    /// cursor past the row committed during the outage.
    ///
    /// **Why this holds the fault open on a COUNTER and not a clock.** This test used to sleep a
    /// fixed 1500ms here. Attempt 1 deleted that window on the ground that `PollOutcome` had made
    /// give-up unrepresentable — which was false: `PollOutcome` constrains `poll_once`, the STEP.
    /// The driver LOOP is separate, and it is where give-up lives. With the window gone, a driver
    /// that returned after 10 consecutive failures passed all 270 tests, and the detection floor
    /// fell from ~20 poll passes to 4.
    ///
    /// Restoring the sleep would restore the original weakness with it: a give-up budget above the
    /// window still passes, and every run on every machine pays the full window. Waiting on
    /// `consecutive_failures` instead kills any budget below 25, returns as soon as the counter
    /// arrives, and makes the health counters load-bearing rather than decorative.
    #[tokio::test]
    async fn tailer_survives_a_transient_read_error() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first, holding the health Arc: the driver's liveness through the outage
        // is observed on `consecutive_failures`, so this test cannot use `spawn_ignoring_health`.
        let (tx, _rx) = broadcast::channel(64);
        let health = Arc::new(TailerHealth::default());
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone(), Arc::clone(&health));

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        await_ready(ready).await;

        // Commit seq 1 before the outage and wait for it. Readiness alone proves only that the
        // mark resolved, NOT that a poll pass ever ran, so without this publication the silence
        // window below would prove nothing — the vacuity class this task keeps failing on. The
        // seq is absolute, and it also leaves the cursor at 1.
        assert_publishes_exactly(&pool, &mut subscriber, 1).await;

        // Induce transient failure via table rename (fires high_water_mark, outer arm)
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        // Commit a row DURING the outage (against the renamed table, the only reachable name).
        // Its silence below is what proves the fault actually fired, and its arrival after the
        // repair is what proves the cursor did not advance past it.
        let outage_row = commit_one_to_hidden_journal(&pool).await;
        assert_eq!(outage_row.seq, 2, "outage row should be seq 2");

        // Hold the outage open until the DRIVER has demonstrably run at least 25 consecutive
        // failing passes and stayed in its loop through every one of them. This is the assertion
        // that the deleted 1500ms window was standing in for, made directly rather than inferred
        // from elapsed time — and it fires as soon as the counter arrives.
        await_consecutive_failures(&health, REQUIRED_CONSECUTIVE_FAILURES).await;

        // Nothing may have published across the WHOLE outage — every one of those 25+ passes, not
        // merely a leading 225ms slice of them.
        assert_nothing_published(
            &mut subscriber,
            "no event may publish during the table-hidden outage",
        );

        // Repair: rename the table back (SQLite auto-reprepares on SQLITE_SCHEMA)
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        // The tailer should still be running (did not end on the error)
        assert!(
            !tailer_handle.is_finished(),
            "tailer should survive the transient read error"
        );

        // The cursor must not have advanced during the outage, so the row committed while the
        // table was hidden is still owed to subscribers. Had the cursor advanced past the outage,
        // it would be lost forever.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                assert_eq!(
                    delivered(&ev),
                    outage_row,
                    "should publish the outage row, body and all, after recovery; cursor did not \
                     advance past error"
                );
            }
            None => {
                panic!("expected to receive the outage row after recovery from transient error")
            }
        }

        tailer_handle.abort();
    }

    #[tokio::test]
    async fn zero_receivers_does_not_stall_the_cursor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first (with no subscribers yet).
        // NOTE the `_` binding, not `_rx`: a receiver live for the whole test makes every `send`
        // succeed, and this test exists precisely to exercise the zero-receiver `send` error path.
        let (tx, _) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // Readiness fixes the cursor at 0, so the three rows below are provably ABOVE it and the
        // tailer must therefore attempt those sends into zero receivers. Attempt 4 depended on a
        // 10ms gap for this: if the initial mark read landed after the commits the tailer started
        // at their mark, never attempted a single zero-receiver send, and the mutation this test
        // exists to catch survived.
        await_ready(ready).await;

        // Prove the poll loop is running, then REMOVE the receiver. The probe-drop is kept from
        // attempt 4's fix: dropping does not permanently close the channel, because tokio resets
        // `tail.closed` at the next `subscribe()` once `rx_cnt` has fallen to 0.
        {
            let mut probe_rx = tx.subscribe();
            assert_publishes_exactly(&pool, &mut probe_rx, 1).await;
            drop(probe_rx);
        }

        // Commit rows with NO subscriber attached. Nothing may observe these, so their identities
        // are irrelevant — only that they take seqs 2-4.
        assert_eq!(
            seqs_of(&commit_batch(&pool, 3).await),
            vec![2, 3, 4],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        // Give the tailer time to poll and advance the cursor despite zero receivers.
        //
        // This is the ONE fixed gap left in the suite and it is irreducible: readiness does NOT
        // mean "the tailer has processed everything committed so far", and with zero receivers the
        // cursor is unobservable by construction, so there is nothing to poll for. Only observing a
        // publication proves a pass happened, and the probe-receiver drop above already does that
        // for the cursor's starting position.
        //
        // Left at the value attempt 4 validated (4 * TAIL_INTERVAL) rather than lengthened: this is
        // not a defect today, because exceeding the gap flips the assertion below from 5 to 2 and
        // FAILS loudly rather than passing vacuously.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // NOW subscribe for the first time
        let mut subscriber = tx.subscribe();

        // Commit one more row
        let committed = commit_one(&pool).await;
        assert_eq!(
            committed.seq, 5,
            "the journal assigned an unexpected seq; this test's absolute expectation is stale"
        );

        // Collect published events using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut received = vec![];
        for _ in 0..1 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => received.push(delivered(&ev)),
                None => break,
            }
        }

        // The FIRST event must be the row committed after subscribing (seq 5: the probe took 1 and
        // the zero-receiver window took 2-4), carrying that row's body. Had the cursor stalled on
        // the zero-receiver send errors, seqs 2-4 would be re-sent now that a receiver has
        // attached, and the first event would be seq 2 instead.
        assert_eq!(
            received,
            vec![committed],
            "tailer should have advanced its cursor despite zero receivers; only the new row, with \
             its own body, should arrive"
        );

        // ...and nothing further arrives (a stalled cursor would still be draining the backlog)
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(ev)) => panic!(
                "only the newly committed row should arrive; also got seq {}",
                ev.seq
            ),
            _ => {
                // Expected: nothing else
            }
        }

        tailer_handle.abort();
    }

    /// The INNER-arm twin of `tailer_survives_a_transient_read_error`: the high-water mark reads
    /// fine, but `read_range` fails on a corrupt payload. Same structure, same reasoning — the
    /// fault is held open on `consecutive_failures` rather than on a clock, for the reasons set out
    /// on that test.
    #[tokio::test]
    async fn a_failed_read_does_not_end_the_loop_or_advance_the_cursor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first, holding the health Arc (see the sibling test): driver liveness
        // through the outage is observed on `consecutive_failures`, not inferred from a sleep.
        let (tx, _rx) = broadcast::channel(64);
        let health = Arc::new(TailerHealth::default());
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone(), Arc::clone(&health));

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        await_ready(ready).await;

        // Commit seq 1 and wait for it: the poll loop is provably running and its cursor is at 1.
        // Readiness alone would not establish that, and without it the silence window below is
        // unattributable.
        assert_publishes_exactly(&pool, &mut subscriber, 1).await;

        // Commit the next row and corrupt it in the SAME transaction, so it is never visible in a
        // readable state. Corrupting it in a separate statement leaves a window in which the tailer
        // can publish the row before the UPDATE lands, which would fail the silence check below.
        let corrupt_seq = {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let seq = event_journal::append(&mut *tx_write, &event).await.unwrap();
            // Corrupt the payload so read_range fails at serde_json::from_str (inner arm)
            sqlx::query("UPDATE event_journal SET payload = '{not json' WHERE seq = ?")
                .bind(seq)
                .execute(&mut *tx_write)
                .await
                .unwrap();
            tx_write.commit().await.unwrap();
            seq
        };
        assert_eq!(corrupt_seq, 2, "corrupt row should be seq 2");

        // Hold the corruption in place until the driver has demonstrably run at least 25
        // consecutive failing passes — all of them failing at `serde_json::from_str` inside
        // `read_range` — and stayed in its loop through every one. Same substitution as the sibling
        // test: an observable the driver must actually produce, in place of a wall-clock window
        // whose only claim was about elapsed time.
        await_consecutive_failures(&health, REQUIRED_CONSECUTIVE_FAILURES).await;

        // Nothing may have published across the WHOLE corruption window.
        assert_nothing_published(
            &mut subscriber,
            "no event may publish while the payload is corrupted",
        );

        // Repair: restore the payload with valid JSON.
        //
        // The repaired body is what the delivered event is asserted against, not the original. The
        // original is unobservable by construction — it never existed in a readable state — and the
        // repaired form is the stronger claim anyway: it pins the delivered body to the journal row
        // AS IT STANDS AT READ TIME, so a tailer serving a cached or invented payload fails here
        // even though the seq is right.
        let repaired = {
            let task_id = Uuid::new_v4();
            let project_id = Uuid::new_v4();
            let event = NodeEvent::TaskCreated {
                task_id,
                project_id,
            };
            let payload = serde_json::to_string(&event).unwrap();
            sqlx::query("UPDATE event_journal SET payload = ? WHERE seq = ?")
                .bind(payload)
                .bind(corrupt_seq)
                .execute(&pool)
                .await
                .unwrap();
            RowId {
                seq: corrupt_seq,
                task_id,
                project_id,
            }
        };

        // Verify the tailer is still running
        assert!(
            !tailer_handle.is_finished(),
            "tailer should continue running after a read error"
        );

        // Verify the corrupted row is now published (proving cursor did not advance)
        // Use deadline-based polling for the positive assertion.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                assert_eq!(
                    delivered(&ev),
                    repaired,
                    "should publish the repaired row, carrying the repaired body; if the cursor \
                     advanced during the error, this would be skipped"
                );
            }
            None => panic!("expected to receive the repaired row after recovery from read error"),
        }

        tailer_handle.abort();
    }

    /// The FULL body of a delivered or committed event, as JSON, paired with its seq.
    ///
    /// `RowId` cannot express this. It carries `task_id`/`project_id` only, and `delivered()`
    /// PANICS on any variant other than `TaskCreated` — so the helpers above cannot state an
    /// expectation about `exit_code`, `old_status`/`new_status`, `reason`, `executor` or
    /// `entity_count` at all. Comparing serialized JSON compares every field of every variant,
    /// including the serde `type` tag, and needs no `PartialEq` on `NodeEvent`.
    fn body_of(ev: &SequencedEvent) -> (i64, serde_json::Value) {
        (ev.seq, serde_json::to_value(&ev.event).unwrap())
    }

    /// The serde tag of a variant, via a match that is deliberately EXHAUSTIVE — no `_` arm.
    ///
    /// This is the drift guard for the finding below: adding a tenth `NodeEvent` variant fails to
    /// COMPILE here, forcing `one_of_every_variant` to be extended rather than silently
    /// reintroducing the blind spot for the new variant. `NodeEvent::event_type()` cannot serve
    /// this purpose — it lives in `crates/db`, so a new variant breaks that crate, not this test.
    fn variant_tag(event: &NodeEvent) -> &'static str {
        match event {
            NodeEvent::TaskCreated { .. } => "task_created",
            NodeEvent::TaskStatusChanged { .. } => "task_status_changed",
            NodeEvent::TaskDeleted { .. } => "task_deleted",
            NodeEvent::AttemptStarted { .. } => "attempt_started",
            NodeEvent::AttemptFinished { .. } => "attempt_finished",
            NodeEvent::AttemptFailed { .. } => "attempt_failed",
            NodeEvent::HiveConnected { .. } => "hive_connected",
            NodeEvent::HiveDisconnected { .. } => "hive_disconnected",
            NodeEvent::ReconcileCompleted { .. } => "reconcile_completed",
        }
    }

    /// One of every `NodeEvent` variant, each carrying distinct field values so that no variant's
    /// body can be satisfied by another's.
    fn one_of_every_variant() -> Vec<NodeEvent> {
        use db::models::task::TaskStatus;
        vec![
            NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            },
            NodeEvent::TaskStatusChanged {
                task_id: Uuid::new_v4(),
                old_status: TaskStatus::Todo,
                new_status: TaskStatus::InProgress,
            },
            NodeEvent::TaskDeleted {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            },
            NodeEvent::AttemptStarted {
                task_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                execution_process_id: Uuid::new_v4(),
                executor: "claude".into(),
            },
            NodeEvent::AttemptFinished {
                task_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                execution_process_id: Uuid::new_v4(),
                executor: "codex".into(),
                exit_code: 7,
            },
            NodeEvent::AttemptFailed {
                task_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                execution_process_id: Uuid::new_v4(),
                executor: "droid".into(),
                reason: "worktree vanished".into(),
            },
            NodeEvent::HiveConnected {},
            NodeEvent::HiveDisconnected {
                reason: "socket closed".into(),
            },
            NodeEvent::ReconcileCompleted { entity_count: 42 },
        ]
    }

    /// Commits the caller's events in ONE transaction and returns their full identities in commit
    /// order. Unlike `commit_batch`, the caller chooses the variants.
    async fn commit_events(
        pool: &SqlitePool,
        events: &[NodeEvent],
    ) -> Vec<(i64, serde_json::Value)> {
        let mut tx = pool.begin().await.unwrap();
        let mut rows = Vec::with_capacity(events.len());
        for event in events {
            let seq = event_journal::append(&mut *tx, event).await.unwrap();
            rows.push((seq, serde_json::to_value(event).unwrap()));
        }
        tx.commit().await.unwrap();
        rows
    }

    /// The tailer's contract is "publish the journal's rows", not "publish the journal's
    /// `TaskCreated` rows".
    ///
    /// Every other test in this file — and every test in `mod.rs` — commits `TaskCreated` and
    /// nothing else, and `delivered()` panics on anything else, so the suite was structurally
    /// unable to express an expectation about eight of the nine variants. A tailer that forwarded
    /// only `TaskCreated` while advancing its cursor past `TaskStatusChanged`, `AttemptFinished`,
    /// `HiveDisconnected` and the rest passed all 263 tests. Tasks 006/007/008 of this same plan
    /// emit precisely those other variants, so in production the bus would keep carrying
    /// `TaskCreated` — looking healthy — while every event phase 3 exists to deliver was lost
    /// permanently, the cursor advanced past it.
    ///
    /// Comparing whole serialized bodies rather than `RowId` is load-bearing: `RowId` has no
    /// `exit_code`, `reason`, `executor` or status fields, so even a per-variant `RowId` test would
    /// leave those payload fields free to be fabricated.
    #[tokio::test]
    async fn every_event_variant_is_published_with_its_body_intact() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());
        let mut subscriber = tx.subscribe();
        await_ready(ready).await;

        let events = one_of_every_variant();
        let tags: Vec<&'static str> = events.iter().map(variant_tag).collect();
        let mut distinct = tags.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            tags.len(),
            "one_of_every_variant repeated a variant, so some variant is unasserted: {tags:?}"
        );
        assert_eq!(
            tags.len(),
            9,
            "a NodeEvent variant was added or removed without extending this test"
        );

        // One transaction, so all nine become visible atomically and a single poll pass reads them
        // as one batch: their relative order is observable and a variant-dependent reordering has
        // nowhere to hide either.
        let committed = commit_events(&pool, &events).await;
        assert_eq!(
            committed.iter().map(|r| r.0).collect::<Vec<i64>>(),
            (1..=9).collect::<Vec<i64>>(),
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut received = vec![];
        for _ in 0..committed.len() {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => received.push(body_of(&ev)),
                None => break,
            }
        }

        tailer_handle.abort();

        assert_eq!(
            received, committed,
            "the tailer must publish EVERY journal row, whatever its variant, carrying every field \
             that was committed; a missing entry means a variant was dropped and its cursor \
             advanced past, a differing body means a variant was mangled"
        );
    }

    /// `seq` must be the value the JOURNAL assigned, not the row's position in the batch.
    ///
    /// The two are indistinguishable on a contiguous journal, and no other test ever produces a
    /// non-contiguous one — so a tailer that renumbered its output as `cursor + 1, cursor + 2, …`
    /// passed all 263 tests. Gaps are not hypothetical: `seq` is `INTEGER PRIMARY KEY
    /// AUTOINCREMENT` precisely so deleted seqs are never reused, and `event_journal::compact`'s
    /// stage-2 hard cap deletes the oldest rows ignoring the cursor floor. The tailer has no row in
    /// `trigger_cursors`, so that floor does not protect it: compaction can delete rows from inside
    /// the tailer's unread window. Under positional numbering every subsequent event is published
    /// under another row's seq — and since `seq` is exactly what consumers persist as their cursor,
    /// they would resume from the wrong place forever.
    ///
    /// The gap is punched INSIDE the committing transaction, so no intermediate state is ever
    /// visible: there is no window in which the tailer could have read the deleted rows.
    #[tokio::test]
    async fn a_gap_in_the_journal_does_not_renumber_the_rows_after_it() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());
        let mut subscriber = tx.subscribe();
        await_ready(ready).await;

        let mut rows = Vec::new();
        {
            let mut tx_write = pool.begin().await.unwrap();
            for _ in 0..4 {
                let task_id = Uuid::new_v4();
                let project_id = Uuid::new_v4();
                let event = NodeEvent::TaskCreated {
                    task_id,
                    project_id,
                };
                let seq = event_journal::append(&mut *tx_write, &event).await.unwrap();
                rows.push(RowId {
                    seq,
                    task_id,
                    project_id,
                });
            }
            sqlx::query("DELETE FROM event_journal WHERE seq IN (?, ?)")
                .bind(rows[0].seq)
                .bind(rows[2].seq)
                .execute(&mut *tx_write)
                .await
                .unwrap();
            tx_write.commit().await.unwrap();
        }

        let survivors = vec![rows[1], rows[3]];
        assert_eq!(
            seqs_of(&survivors),
            vec![2, 4],
            "the surviving rows must be seqs 2 and 4, or this test is not exercising a gap"
        );
        assert_eq!(
            event_journal::high_water_mark(&pool).await.unwrap(),
            4,
            "MAX(seq) must remain 4 despite the deletions, or the tailer never reads seq 4 at all"
        );

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut received = vec![];
        for _ in 0..survivors.len() {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => received.push(delivered(&ev)),
                None => break,
            }
        }

        tailer_handle.abort();

        assert_eq!(
            received, survivors,
            "the surviving rows must be published under the seqs the JOURNAL gave them (2 and 4); \
             receiving them as 1 and 2 means the tailer numbered its output by batch position, and \
             every consumer cursor built on those numbers would be permanently wrong"
        );
    }

    /// A batch far larger than any per-pass budget must be published in full.
    ///
    /// No other test commits more than three rows in one transaction, so nothing bounded the work a
    /// single pass may do: a tailer that published only the first 64 rows of a pass and then
    /// advanced its cursor to the mark passed all 263 tests, losing everything past the 64th row of
    /// any bulk write. Bulk writes are exactly what this journal sees — a cascading task delete or
    /// a reconcile emits many rows in one transaction — and the sharpest trigger is this file's own
    /// `tailer_survives_a_transient_read_error`, whose whole point is that the cursor does NOT
    /// advance during an outage, which guarantees a large catch-up batch on recovery.
    ///
    /// NOTE the name is the one this task dictated and is slightly misleading: the channel is sized
    /// 4x the batch ON PURPOSE. At capacity 64 a 200-row batch would hand the receiver
    /// `RecvError::Lagged`, which `poll_for_event` reports as `None`, and the test would fail for a
    /// reason that has nothing to do with the tailer. What this pins is "no per-pass cap", not
    /// buffer-overrun behaviour.
    #[tokio::test]
    async fn a_batch_larger_than_the_broadcast_buffer_is_published_whole() {
        const N: usize = 200;

        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let (tx, _rx) = broadcast::channel(N * 4);
        let (tailer_handle, ready) = spawn_ignoring_health(pool.clone(), tx.clone());
        let mut subscriber = tx.subscribe();
        await_ready(ready).await;

        let committed = commit_batch(&pool, N).await;
        assert_eq!(
            seqs_of(&committed),
            (1..=N as i64).collect::<Vec<i64>>(),
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut received = vec![];
        for _ in 0..N {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => received.push(delivered(&ev)),
                None => break,
            }
        }

        tailer_handle.abort();

        assert_eq!(
            received.len(),
            N,
            "the tailer published {} of {N} rows committed in one transaction; a per-pass cap that \
             advances the cursor to the mark loses the remainder permanently",
            received.len()
        );
        assert_eq!(
            received, committed,
            "every row of a large batch must arrive, in seq order, carrying its own body"
        );
    }

    /// Property 1 binds the ERROR path as well as the happy path: when the FIRST `high_water_mark`
    /// call fails, the tailer must RETRY until a mark is obtainable. Falling back to 0 — attempt
    /// 1's actual bug — makes the tailer replay the entire journal onto the live channel.
    ///
    /// The fault has to be in place BEFORE the tailer is spawned, which is what distinguishes this
    /// from `tailer_survives_a_transient_read_error`: that one's ALTER TABLE only fires after the
    /// initial retry loop has already succeeded, so it cannot see this path at all.
    #[tokio::test]
    async fn tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Hide the table BEFORE spawning, so the tailer's very first high_water_mark call fails
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, mut ready) = spawn_ignoring_health(pool.clone(), tx.clone());

        // Subscribe before anything can be published, so a replay cannot slip past unobserved
        let mut subscriber = tx.subscribe();

        // Commit three rows while the table is renamed away. A tailer that retries its initial
        // mark starts ABOVE these; a tailer that fell back to 0 replays every one of them.
        for _ in 0..3 {
            commit_one_to_hidden_journal(&pool).await;
        }

        // The outage window must be observably silent, AND it must be long enough that "the tailer
        // is still retrying" is a claim about the retry loop rather than about the clock.
        //
        // 8000ms, not the 750ms this test shipped with and not the 4000ms floor the amendment
        // named. 750ms is about four retries at the 100/200/400/800/800… backoff, which is why a
        // mutant that gave up with `break 0` after ten retries passed this test three times over.
        // The arithmetic for that mutant: the sleeps preceding its tenth attempt are
        // 100+200+400+800+800*5 = 5500ms, so ANY window below ~5.5s lets it recover during the
        // repair below and stay completely invisible — the 4000ms floor included. 8000ms clears
        // 5500ms by ~45%, and the margin only has to be one-sided: machine load stretches the
        // mutant's sleeps, it never shortens them.
        //
        // At 8000ms a correct tailer makes ~12 retries here. The residual is declared in the
        // decisions-ledger and is not claimed away: no finite window excludes an arbitrarily large
        // finite give-up budget.
        match tokio::time::timeout(std::time::Duration::from_millis(8000), subscriber.recv()).await
        {
            Ok(Ok(ev)) => panic!(
                "nothing may publish while the journal table is hidden; got seq {}",
                ev.seq
            ),
            _ => {
                // Expected: the journal is unreadable, so there is nothing to publish
            }
        }

        // Readiness must NOT have fired: a mark is genuinely unobtainable while the table is
        // hidden, so a tailer that has signalled is a tailer that invented a cursor. This catches
        // the fall-back-to-0 bug at its source rather than through its downstream replay.
        assert!(
            matches!(ready.try_recv(), Err(oneshot::error::TryRecvError::Empty)),
            "the tailer signalled readiness while the journal table was unreadable, so it did not \
             retry the initial high-water mark — it fabricated one"
        );

        // The retry loop must still be retrying, not have exited the task
        assert!(
            !tailer_handle.is_finished(),
            "tailer must retry the initial high-water mark, not give up"
        );

        // Repair: rename back. The retry now succeeds and resolves to a mark of 3.
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        // Only now can the mark resolve, so only now can readiness fire
        await_ready(ready).await;

        // A correct tailer publishes NOTHING already committed and picks up at exactly seq 4.
        // Pinned to an exact value, not `base > 3`: the one-sided form passes under the
        // `break mark + 1` skip mutation, which resumes at 5. Any of seqs 1-3 arriving instead is
        // the fall-back-to-0 replay.
        assert_publishes_exactly(&pool, &mut subscriber, 4).await;

        tailer_handle.abort();
    }

    /// The structural guarantee task 016 exists to add: `PollOutcome` has NO variant that ends
    /// the loop, so 50 idle passes and 50 failures are indistinguishable from 5 in the type
    /// system. Drives `poll_once` directly — no spawned task, no `sleep` anywhere — so this runs
    /// in microseconds and is immune to machine load, unlike every prior "does not give up" test
    /// in this file.
    #[tokio::test]
    async fn a_poll_step_can_never_terminate_the_loop() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let (tx, _rx) = broadcast::channel(64);
        let health = TailerHealth::default();

        // Against an EMPTY journal, every call is Idle and the cursor never moves.
        let mut cursor = 0i64;
        for i in 0..50 {
            let outcome = poll_once(&pool, &tx, &mut cursor, &health).await;
            assert_eq!(
                outcome,
                PollOutcome::Idle,
                "pass {i} against an empty journal must report Idle, not {outcome:?}"
            );
        }
        assert_eq!(cursor, 0, "an idle poll must never move the cursor");

        // Against an UNREADABLE journal, every call is Failed and the cursor it is handed comes
        // back UNCHANGED.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();
        let mut failing_cursor = 0i64;
        for i in 0..50 {
            let outcome = poll_once(&pool, &tx, &mut failing_cursor, &health).await;
            assert_eq!(
                outcome,
                PollOutcome::Failed,
                "pass {i} against an unreadable journal must report Failed, not {outcome:?}"
            );
            assert_eq!(
                failing_cursor, 0,
                "pass {i}: a Failed poll must never advance the cursor it was handed"
            );
        }
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        // Against a journal with rows above the cursor, the pass reports Published{count} and
        // advances the cursor to the last published seq.
        let committed = commit_batch(&pool, 3).await;
        let mut published_cursor = 0i64;
        let outcome = poll_once(&pool, &tx, &mut published_cursor, &health).await;
        match outcome {
            PollOutcome::Published { count } => assert_eq!(
                count, 3,
                "a pass reading 3 new rows must report a Published count of 3"
            ),
            other => panic!("expected Published, got {other:?}"),
        }
        assert_eq!(
            published_cursor,
            committed.last().unwrap().seq,
            "the cursor must advance to the last published seq"
        );
    }

    /// The IDLE half of the driver guard, and the one attempt 2 did not have.
    ///
    /// `PollOutcome` makes give-up unrepresentable inside `poll_once`; it does not constrain the
    /// DRIVER, which is where the loop lives. Attempt 2 guarded the driver on
    /// `consecutive_failures`, a counter that moves only on `PollOutcome::Failed` — so a driver
    /// that abandoned the loop after 40 consecutive `Idle` passes (3.0s of journal quiet, the
    /// ordinary state of an idle node) passed all 272 tests, as did one that backed off to a 60s
    /// poll and therefore never ended at all.
    ///
    /// This test needs NO fault: a quiet journal is the condition. It waits on `polls_total`, which
    /// advances whatever the outcome, so the same wait that guards the faulted sibling guards this.
    #[tokio::test]
    async fn the_driver_keeps_polling_a_quiet_journal() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let (tx, _rx) = broadcast::channel(64);
        let health = Arc::new(TailerHealth::default());
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone(), Arc::clone(&health));
        let mut subscriber = tx.subscribe();

        await_ready(ready).await;

        // A tripwire, NOT the thing the kill depends on — see `MAX_CADENCE_BASELINE_POLLS` for the
        // arithmetic. `REQUIRED_POLL_CLIMB` is sized against a baseline of 0 because that is the
        // worst case, and a larger baseline only makes the kill easier.
        let polls_before = health.polls_total.load(Ordering::Relaxed);
        assert!(
            polls_before < MAX_CADENCE_BASELINE_POLLS,
            "polls_total already read {polls_before} at readiness, before this tailer's driver \
             had run. The climb below is still sound — its target scales with the baseline — but a \
             counter pre-advanced by something other than this driver means these cadence tests are \
             no longer measuring what their names say"
        );

        await_polls_climb(&health, REQUIRED_POLL_CLIMB, CADENCE_DEADLINE).await;

        // Quiet means quiet: those 50 passes had nothing to publish and must not have invented
        // anything. This is what makes the passes above provably IDLE ones rather than merely
        // counted ones.
        assert_nothing_published(
            &mut subscriber,
            "no event may publish while the journal is empty",
        );

        // Diagnostic, NOT a kill: a give-up budget between REQUIRED_POLL_CLIMB and whatever the
        // deadline permits passes the climb above with the task still alive here. The climb is the
        // guard; this only makes an unexpected death report itself in the obvious place.
        assert!(
            !tailer_handle.is_finished(),
            "the tailer task ended while polling a quiet journal"
        );

        tailer_handle.abort();
    }

    /// The FAULTED half of the driver guard: the same `polls_total` climb, held under a fault that
    /// makes every pass fail, then repaired.
    ///
    /// `await_consecutive_failures` (kept, on the two tests that carry it) pins the failure-path
    /// budget at a named site. This one asks a different question — that the driver keeps its
    /// CADENCE while failing — and it is the wait that would still fire if `consecutive_failures`
    /// itself were miscounted, which is the hazard attempt 2's guard introduced: an over-counting
    /// `fetch_add(3)` reaches a target of 25 after 9 real passes, silently dropping the advertised
    /// detection floor. `polls_total` is pinned exactly by
    /// `poll_once_pins_every_counter_to_an_exact_value`, so this wait cannot be shortened the same
    /// way.
    ///
    /// The repair assertion is the other half: surviving the fault is worth nothing if the tailer
    /// comes back deaf.
    #[tokio::test]
    async fn the_driver_keeps_polling_a_journal_it_cannot_read() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let (tx, _rx) = broadcast::channel(64);
        let health = Arc::new(TailerHealth::default());
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone(), Arc::clone(&health));
        let mut subscriber = tx.subscribe();

        await_ready(ready).await;

        // Hold the fault open for the whole climb: every pass below fails in `high_water_mark`.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        // Same tripwire as the sibling test, and the same non-dependence of the kill on it. Read
        // AFTER the rename, so the passes counted below are the failing ones.
        let polls_before = health.polls_total.load(Ordering::Relaxed);
        assert!(
            polls_before < MAX_CADENCE_BASELINE_POLLS,
            "polls_total already read {polls_before} when the fault was induced, before this \
             tailer's driver had meaningfully run. The climb below is still sound — its target \
             scales with the baseline — but a counter pre-advanced by something other than this \
             driver means these cadence tests are no longer measuring what their names say"
        );

        await_polls_climb(&health, REQUIRED_POLL_CLIMB, CADENCE_DEADLINE).await;

        assert_nothing_published(
            &mut subscriber,
            "no event may publish while the journal table is renamed away",
        );

        // Repair, and require the tailer to come back: a driver that kept its counter climbing but
        // stopped publishing would pass the climb alone.
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        // ABSOLUTE seq 1: nothing was ever committed to this journal before now, so a tailer that
        // skipped ahead during the outage — or replayed from 0 — cannot land here by accident.
        assert_publishes_exactly(&pool, &mut subscriber, 1).await;

        tailer_handle.abort();
    }

    /// Pins every counter to an EXACT value by driving `poll_once` a known number of times. No
    /// spawned task, no `sleep`, so this runs in microseconds and is immune to machine load.
    ///
    /// This is what makes the cadence tests above sound. Their waits are only as trustworthy as the
    /// counters they read, and attempt 2 demonstrated the failure mode: a production
    /// `fetch_add(3)` in place of `fetch_add(1)` let `await_consecutive_failures(25)` return after
    /// 9 real failing passes, dropping the advertised detection floor from 25 to 9 with the whole
    /// suite green — and the only visible symptom was that the tests got FASTER.
    ///
    /// Every assertion here is exact equality. `>=` is precisely what let the over-counting
    /// mutation live: a counter that runs ahead satisfies every one-sided bound in the file.
    #[tokio::test]
    async fn poll_once_pins_every_counter_to_an_exact_value() {
        const IDLE_PASSES: u64 = 7;
        const FAILED_PASSES: u64 = 5;

        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let (tx, _rx) = broadcast::channel(64);
        let health = TailerHealth::default();
        let mut cursor = 0i64;

        // --- Idle passes: polls_total counts them, nothing else moves. ---
        for i in 0..IDLE_PASSES {
            let outcome = poll_once(&pool, &tx, &mut cursor, &health).await;
            assert_eq!(outcome, PollOutcome::Idle, "pass {i} on an empty journal");
        }
        assert_eq!(
            health.polls_total.load(Ordering::Relaxed),
            IDLE_PASSES,
            "polls_total must equal the number of passes driven, exactly — not at least"
        );
        assert_eq!(
            health.consecutive_failures.load(Ordering::Relaxed),
            0,
            "an idle pass must leave consecutive_failures at 0"
        );
        assert_eq!(
            health.last_published_seq.load(Ordering::Relaxed),
            0,
            "an idle pass publishes nothing and must not touch last_published_seq"
        );

        // --- Failed passes: consecutive_failures counts them ONE per pass, and polls_total keeps
        // counting too. The contract is "every pass, whatever its outcome"; moving the increment
        // into the Ok(mark) arm freezes polls_total at IDLE_PASSES here.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();
        for i in 0..FAILED_PASSES {
            let outcome = poll_once(&pool, &tx, &mut cursor, &health).await;
            assert_eq!(
                outcome,
                PollOutcome::Failed,
                "pass {i} against an unreadable journal"
            );
        }
        assert_eq!(
            health.polls_total.load(Ordering::Relaxed),
            IDLE_PASSES + FAILED_PASSES,
            "a FAILED pass must still increment polls_total: the counter is 'passes attempted, \
             whatever their outcome', and it is the observable the driver-cadence tests wait on \
             during exactly this fault"
        );
        assert_eq!(
            health.consecutive_failures.load(Ordering::Relaxed),
            FAILED_PASSES,
            "consecutive_failures must equal the number of failed passes EXACTLY. Over-counting \
             makes every wait on this counter return early — a fetch_add(3) turns a wait for 25 \
             into 9 real passes — and no >= assertion anywhere in this file can see it"
        );
        assert_eq!(
            health.last_published_seq.load(Ordering::Relaxed),
            0,
            "a failed pass publishes nothing and must not touch last_published_seq"
        );
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        // --- One pass publishing a MULTI-ROW batch. The multi-row shape is the point: every other
        // health-observing test in this file publishes one row per pass, so count == 1 and first
        // == last on every pass, and storing the batch's FIRST seq is indistinguishable from
        // storing its last.
        let committed = commit_batch(&pool, 3).await;
        assert_eq!(
            seqs_of(&committed),
            vec![1, 2, 3],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );
        let outcome = poll_once(&pool, &tx, &mut cursor, &health).await;
        assert_eq!(
            outcome,
            PollOutcome::Published { count: 3 },
            "one pass over a 3-row batch must report all three"
        );
        assert_eq!(
            health.polls_total.load(Ordering::Relaxed),
            IDLE_PASSES + FAILED_PASSES + 1,
            "a publishing pass counts as exactly one pass"
        );
        assert_eq!(
            health.consecutive_failures.load(Ordering::Relaxed),
            0,
            "ONE successful pass must reset consecutive_failures to 0, not decrement it"
        );
        assert_eq!(
            health.last_published_seq.load(Ordering::Relaxed),
            3,
            "last_published_seq must equal the LAST seq of the batch (3), not its first (1): the \
             batch landed in a single pass, so first and last differ here and only here"
        );
    }

    /// Makes the tailer's liveness directly OBSERVABLE via counters, rather than inferred from
    /// timing side-effects. `polls_total` proves the loop is running at all; `consecutive_failures`
    /// proves an outage is happening and that recovery is detected; `last_published_seq` proves
    /// what a subscriber should already know from the broadcast channel, but now from a
    /// health-check surface that does not require a live subscriber.
    #[tokio::test]
    async fn tailer_health_advances_while_polling_and_records_failures() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let (tx, _rx) = broadcast::channel(64);
        let health = Arc::new(TailerHealth::default());
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone(), Arc::clone(&health));
        let mut subscriber = tx.subscribe();
        await_ready(ready).await;

        // Assert the counters AS COUNTERS, across SEVERAL rows and SEVERAL passes.
        //
        // Publishing exactly one row — which is what this test used to do — makes every counter
        // indistinguishable from the literal `1`. Both `polls_total` frozen at 1 and
        // `last_published_seq` hardcoded to 1 survived the entire 270-test suite on attempt 1. A
        // liveness signal that cannot be falsified is worse than none: it converts "unknown" into
        // "healthy", which is the failure mode these counters exist to remove.
        let polls_before = health.polls_total.load(Ordering::Relaxed);

        // Four rows, each committed only after the previous one has been DELIVERED, so each
        // necessarily lands on a later poll pass than the last.
        for expected_seq in 1..=4 {
            assert_publishes_exactly(&pool, &mut subscriber, expected_seq).await;
        }

        let polls_after = health.polls_total.load(Ordering::Relaxed);
        assert!(
            polls_after > polls_before && polls_after >= polls_before + 3,
            "polls_total must CLIMB STRICTLY as passes run: it read {polls_before} before four \
             rows were published across four separate passes and {polls_after} after. Rows 2, 3 \
             and 4 were each committed only after the previous row was delivered, so each was \
             published by a pass whose increment provably happened after the first reading — at \
             least 3 increments are owed. (Row 1's pass may have incremented before the first \
             reading and is not counted.) A counter frozen at any constant reports no climb here."
        );
        assert_eq!(
            health.consecutive_failures.load(Ordering::Relaxed),
            0,
            "a successful publish must leave consecutive_failures at 0"
        );
        assert_eq!(
            health.last_published_seq.load(Ordering::Relaxed),
            4,
            "last_published_seq must equal the seq of the LAST row published (4), not the first \
             and not any constant"
        );

        // Induce an outage: consecutive_failures must climb.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let climb_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if health.consecutive_failures.load(Ordering::Relaxed) >= 3 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < climb_deadline,
                "consecutive_failures did not climb within 30s of an induced outage"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Repair: consecutive_failures must return to 0.
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        let reset_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if health.consecutive_failures.load(Ordering::Relaxed) == 0 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < reset_deadline,
                "consecutive_failures did not reset to 0 after repair within 30s"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        tailer_handle.abort();
    }

    /// `last_published_seq` must start at the RESOLVED INITIAL MARK, not at 0.
    ///
    /// The cursor starts at the high-water mark, so on a non-empty journal a `last_published_seq`
    /// of 0 is indistinguishable from "nothing has ever been published" — the exact
    /// green-while-dead confusion this task's counters exist to remove — right up until the first
    /// post-start publish, which on a quiet node may never come.
    ///
    /// Asserted immediately after readiness, which is sound: readiness fires once the mark is fixed
    /// and BEFORE the first poll pass, and every pass that follows on this journal is `Idle`
    /// (nothing is committed after readiness here), which by construction leaves
    /// `last_published_seq` alone. So the value observed is the initialisation and nothing else.
    #[tokio::test]
    async fn tailer_health_starts_at_the_resolved_initial_mark_not_zero() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Three rows already in the journal BEFORE the tailer starts: the mark resolves to 3.
        assert_eq!(
            seqs_of(&commit_batch(&pool, 3).await),
            vec![1, 2, 3],
            "the journal assigned unexpected seqs; this test's absolute expectations are stale"
        );

        let (tx, _rx) = broadcast::channel(64);
        let health = Arc::new(TailerHealth::default());
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone(), Arc::clone(&health));

        await_ready(ready).await;

        assert_eq!(
            health.last_published_seq.load(Ordering::Relaxed),
            3,
            "last_published_seq must be initialised to the resolved initial mark (3), not 0; a 0 \
             here reports 'nothing ever published' on a journal that already holds three rows"
        );

        tailer_handle.abort();
    }
}
