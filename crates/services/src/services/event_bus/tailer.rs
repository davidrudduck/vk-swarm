//! Journal tailer that publishes committed rows onto the broadcast channel.
//!
//! The tailer implements "journal-first, broadcast-second" publication:
//! - Reads the journal periodically (tail interval bounded)
//! - Publishes newly-committed rows to the broadcast channel
//! - Consumers subscribe to the broadcast live, plus replay-to-live via subscribe_from
//! - The journal is the source of truth; broadcast is an optimization

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

        loop {
            // Get the current high-water mark
            match event_journal::high_water_mark(&pool).await {
                Ok(mark) => {
                    // Read all rows in (last_published, mark]
                    // read_range returns Vec<SequencedEvent> with already-deserialized events
                    match event_journal::read_range(&pool, last_published, mark).await {
                        Ok(seq_events) => {
                            for seq_ev in seq_events {
                                // Publish to the broadcast channel.
                                // Ignore send errors — they mean zero receivers (normal idle state).
                                // Advance last_published regardless.
                                let _ = sender.send(seq_ev.clone());
                                last_published = seq_ev.seq;
                            }
                            debug!(last_published, "tailer pass completed");
                        }
                        Err(e) => {
                            // Read error: log and retry without advancing
                            warn!(error = ?e, "event journal tail read failed; retrying");
                        }
                    }
                }
                Err(e) => {
                    // Failed to fetch high-water mark: log and retry
                    warn!(error = ?e, "failed to fetch high-water mark; retrying");
                }
            }

            // Wait for the next poll interval
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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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
        let (tailer_handle2, ready2) = spawn(pool.clone(), tx2.clone());

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

    #[tokio::test]
    async fn tailer_survives_a_transient_read_error() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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

        // Hold the outage for at least 1500ms — 20 poll attempts at TAIL_INTERVAL = 75ms.
        //
        // This length is the whole point of the wait, not a "give it a moment" margin. The
        // `!is_finished()` assertion below claims the tailer DOES NOT GIVE UP, and an assertion of
        // that shape is only as strong as the outage it survives: at the previous 225ms sleep the
        // tailer faced about six failed passes, so a main loop that returned after ten consecutive
        // failures passed this test twice over — the assertion was wired to the branch but defeated
        // by its window. Twenty attempts exceeds any budget a plausible "add a retry limit" change
        // would use. See the declared residual in the decisions-ledger: no finite wall-clock window
        // can exclude an arbitrarily large finite give-up budget, and this does not claim to.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        // Verify no additional event was published during outage (fault fired).
        // Use a bounded wait (3 * TAIL_INTERVAL) to check for spurious events. This one is
        // deliberately left at 225ms: it proves the FAULT FIRED, which is a different job from the
        // duration wait above, and lengthening it buys nothing.
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(ev)) => panic!(
                "no event should publish during table-hidden outage; got seq {}",
                ev.seq
            ),
            _ => {
                // Expected: nothing published while table is hidden
            }
        }

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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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

    #[tokio::test]
    async fn a_failed_read_does_not_end_the_loop_or_advance_the_cursor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());

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

        // Hold the outage for at least 1500ms — 20 read attempts at TAIL_INTERVAL = 75ms — all of
        // which fail at `serde_json::from_str`.
        //
        // Same reasoning as `tailer_survives_a_transient_read_error`: the `!is_finished()`
        // assertion below claims the loop DOES NOT END on a read error, and at the previous 225ms
        // the tailer only had to survive about six failed reads, so a loop that returned after ten
        // consecutive failures passed this test. The window, not the assertion, was the weak part.
        // The residual is declared in the decisions-ledger.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        // Verify nothing was published during the corruption (fault fired).
        // Use bounded wait (3 * TAIL_INTERVAL) to ensure we're checking real state. Left at 225ms
        // on purpose: this window proves the FAULT FIRED, not that the tailer kept retrying.
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(ev)) => panic!(
                "no event should publish while payload is corrupted; got seq {}",
                ev.seq
            ),
            _ => {
                // Expected: nothing published
            }
        }

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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());
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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());
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
        let (tailer_handle, ready) = spawn(pool.clone(), tx.clone());
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
        let (tailer_handle, mut ready) = spawn(pool.clone(), tx.clone());

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
}
