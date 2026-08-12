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
use tokio::sync::broadcast;
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
/// Returns a `JoinHandle` to stop the tailer cleanly on shutdown.
pub fn spawn(pool: SqlitePool, sender: broadcast::Sender<SequencedEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
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
    })
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

    /// Commits one journal row and returns the seq it was assigned.
    async fn commit_one(pool: &SqlitePool) -> i64 {
        let mut tx = pool.begin().await.unwrap();
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let seq = event_journal::append(&mut *tx, &event).await.unwrap();
        tx.commit().await.unwrap();
        seq
    }

    /// Blocks until a freshly spawned tailer is demonstrably live, returning the seq its cursor
    /// now sits at. Every subsequent commit by the caller is strictly after the tailer's initial
    /// high-water-mark read.
    ///
    /// `tokio::spawn` only SCHEDULES the tailer; its initial `high_water_mark` read can therefore
    /// resolve AFTER rows the test commits immediately afterwards, in which case the tailer
    /// correctly starts above them and publishes nothing — the `left: []` failure that made this
    /// suite flaky under full-crate load. A fixed sleep cannot close that window, because the only
    /// observable proof that the initial read has completed is a publication. So this commits
    /// probe rows until one comes back, which is a happens-before edge rather than a hopeful gap.
    ///
    /// It also DRAINS: it returns only once the NEWEST probe row has been received, so no stale
    /// probe event can be mistaken for a later assertion's event.
    ///
    /// `floor` is the journal high-water mark at the moment the tailer was spawned. A correct
    /// tailer starts there (property 1), so any event at or below it is a history replay.
    async fn probe_until_live(
        pool: &SqlitePool,
        subscriber: &mut tokio::sync::broadcast::Receiver<SequencedEvent>,
        floor: i64,
    ) -> i64 {
        for _ in 0..10 {
            let seq = commit_one(pool).await;
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match poll_for_event(subscriber, deadline).await {
                    Some(ev) if ev.seq == seq => return seq,
                    Some(ev) => assert!(
                        ev.seq > floor,
                        "tailer replayed history: published seq {} at or below its spawn-time high-water mark {}",
                        ev.seq,
                        floor
                    ),
                    None => break,
                }
            }
        }
        panic!("tailer never published a probe row; it never became live");
    }

    /// Commits one journal row directly to the RENAMED journal table.
    ///
    /// `event_journal::append` targets the original table name, so it cannot be used while the
    /// table is renamed away — and the initial-mark outage test needs rows committed DURING the
    /// outage, which is the only way to prove a correct tailer starts above them afterwards.
    async fn commit_one_to_hidden_journal(pool: &SqlitePool) -> i64 {
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let payload = serde_json::to_string(&event).unwrap();
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO event_journal_hidden (event_type, payload) VALUES (?, ?) RETURNING seq",
        )
        .bind(event.event_type())
        .bind(payload)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn tailer_publishes_committed_rows_in_seq_order() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first (high-water mark is 0)
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel BEFORE committing rows
        let mut subscriber = tx.subscribe();

        // Wait until the tailer is provably live rather than sleeping and hoping. Its spawn-time
        // high-water mark is 0, so nothing it publishes may sit at or below that.
        let base = probe_until_live(&pool, &mut subscriber, 0).await;

        // Now append and commit 3 rows
        {
            let mut tx_write = pool.begin().await.unwrap();
            for _ in 0..3 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            }
            tx_write.commit().await.unwrap();
        }

        // Collect the published events using deadline-based polling.
        // This is immune to runtime contention; waits up to deadline for events to arrive.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut seqs = vec![];
        for _ in 0..3 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => seqs.push(ev.seq),
                None => break,
            }
        }

        tailer_handle.abort();

        // The three rows must arrive, consecutive and ascending, immediately after the probe row
        assert_eq!(
            seqs,
            vec![base + 1, base + 2, base + 3],
            "tailer should publish the three committed rows in seq order"
        );
    }

    #[tokio::test]
    async fn tailer_never_publishes_a_rolled_back_row() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Wait until the tailer is provably live (spawn-time high-water mark 0)
        let base = probe_until_live(&pool, &mut subscriber, 0).await;

        // Open a transaction, append an event, and roll back without committing
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

        // Now commit a different row
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            tx_write.commit().await.unwrap();
        }

        // Collect the published event using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                // The rolled-back append released its seq, so the committed row takes it: the very
                // next seq after the probe row. Receiving anything else means the rolled-back row
                // was published too.
                assert_eq!(
                    ev.seq,
                    base + 1,
                    "should only receive the committed row, not the rolled-back one"
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
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Wait until the tailer is provably live (spawn-time high-water mark 0)
        let base = probe_until_live(&pool, &mut subscriber, 0).await;

        // Append and commit 3 journal rows
        {
            let mut tx_write = pool.begin().await.unwrap();
            for _ in 0..3 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            }
            tx_write.commit().await.unwrap();
        }

        // Collect the published events from the first pass using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut first_pass = vec![];
        for _ in 0..3 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => first_pass.push(ev.seq),
                None => break,
            }
        }

        // The first pass MUST actually have published the rows. Without this the whole test is
        // vacuous: a tailer that publishes nothing at all trivially "does not republish".
        assert_eq!(
            first_pass,
            vec![base + 1, base + 2, base + 3],
            "the first pass must publish the three committed rows"
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

        // Append and commit 3 rows
        {
            let mut tx = pool.begin().await.unwrap();
            for _ in 0..3 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            }
            tx.commit().await.unwrap();
        }

        // Create the broadcast channel and start the tailer
        // It will start at high-water mark (3) and won't republish the initial 3 rows
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

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
        let tailer_handle2 = spawn(pool.clone(), tx2.clone());

        // Subscribe to the new tailer
        let mut subscriber2 = tx2.subscribe();

        // Wait until the NEW tailer is provably live before committing the rows under assertion.
        // This is the race that made this test fail with `left: []`: the rows used to be committed
        // immediately after `tokio::spawn`, so the tailer's initial high-water-mark read could
        // resolve after them, start at 5, and correctly never publish 4 and 5.
        // `floor = 3` also asserts the new tailer never replays the 3 pre-existing rows.
        let base = probe_until_live(&pool, &mut subscriber2, 3).await;
        assert!(
            base > 3,
            "new tailer must resume above the pre-existing high-water mark of 3, got {base}"
        );

        // Now append 2 more rows
        {
            let mut tx = pool.begin().await.unwrap();
            for _ in 0..2 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            }
            tx.commit().await.unwrap();
        }

        // Collect the published events using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut seqs = vec![];
        for _ in 0..2 {
            match poll_for_event(&mut subscriber2, deadline).await {
                Some(ev) => seqs.push(ev.seq),
                None => break,
            }
        }

        tailer_handle2.abort();

        // Only the 2 new rows should be published, in order, with no history mixed in
        assert_eq!(
            seqs,
            vec![base + 1, base + 2],
            "new tailer should resume from high-water and publish only new rows"
        );
    }

    #[tokio::test]
    async fn tailer_survives_a_transient_read_error() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Commit one row before the outage and wait until it comes back, which both proves the
        // tailer is live and leaves its cursor at that row.
        let base = probe_until_live(&pool, &mut subscriber, 0).await;

        // Induce transient failure via table rename (fires high_water_mark, outer arm)
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        // Commit a row DURING the outage (against the renamed table, the only reachable name).
        // Its silence below is what proves the fault actually fired, and its arrival after the
        // repair is what proves the cursor did not advance past it.
        let outage_seq = commit_one_to_hidden_journal(&pool).await;
        assert_eq!(outage_seq, base + 1, "outage row should be the next seq");

        // Wait several multiples of TAIL_INTERVAL (75ms * 3 = 225ms) to ensure the tailer
        // has attempted multiple reads and confirmed nothing is published during the outage.
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;

        // Verify no additional event was published during outage (fault fired).
        // Use a bounded wait (3 * TAIL_INTERVAL) to check for spurious events.
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
                    ev.seq, outage_seq,
                    "should publish the outage row after recovery; cursor did not advance past error"
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

        // Start the tailer first (with no subscribers yet)
        let (tx, _) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Commit rows with NO subscriber attached
        {
            let mut tx_write = pool.begin().await.unwrap();
            for _ in 0..3 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            }
            tx_write.commit().await.unwrap();
        }

        // Give the tailer time to poll and advance the cursor despite zero receivers
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // NOW subscribe for the first time
        let mut subscriber = tx.subscribe();

        // Commit one more row
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            tx_write.commit().await.unwrap();
        }

        // Collect published events using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut seqs = vec![];
        for _ in 0..1 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(ev) => seqs.push(ev.seq),
                None => break,
            }
        }

        tailer_handle.abort();

        // Should only receive seq 4 (the new row committed after we subscribed)
        // If the cursor had stalled on the zero-receiver error, seqs 1-3 would be republished
        assert_eq!(
            seqs,
            vec![4],
            "tailer should have advanced its cursor despite zero receivers; only new rows should arrive"
        );
    }

    #[tokio::test]
    async fn a_failed_read_does_not_end_the_loop_or_advance_the_cursor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel
        let mut subscriber = tx.subscribe();

        // Commit one row and wait until it comes back: the tailer is live and its cursor is there
        let base = probe_until_live(&pool, &mut subscriber, 0).await;

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
        assert_eq!(corrupt_seq, base + 1, "corrupt row should be the next seq");

        // Wait for the tailer to attempt multiple reads and fail at deserialization
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;

        // Verify nothing was published during the corruption (fault fired).
        // Use bounded wait (3 * TAIL_INTERVAL) to ensure we're checking real state.
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(ev)) => panic!(
                "no event should publish while payload is corrupted; got seq {}",
                ev.seq
            ),
            _ => {
                // Expected: nothing published
            }
        }

        // Repair: restore the payload with valid JSON
        {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let payload = serde_json::to_string(&event).unwrap();
            sqlx::query("UPDATE event_journal SET payload = ? WHERE seq = ?")
                .bind(payload)
                .bind(corrupt_seq)
                .execute(&pool)
                .await
                .unwrap();
        }

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
                    ev.seq, corrupt_seq,
                    "should publish the repaired row; if the cursor advanced during the error, this would be skipped"
                );
            }
            None => panic!("expected to receive the repaired row after recovery from read error"),
        }

        tailer_handle.abort();
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
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe before anything can be published, so a replay cannot slip past unobserved
        let mut subscriber = tx.subscribe();

        // Commit three rows while the table is renamed away. A tailer that retries its initial
        // mark starts ABOVE these; a tailer that fell back to 0 replays every one of them.
        for _ in 0..3 {
            commit_one_to_hidden_journal(&pool).await;
        }

        // The outage window must be observably silent — several multiples of TAIL_INTERVAL, and
        // longer than the initial retry backoff's first few steps (100/200/400ms).
        match tokio::time::timeout(std::time::Duration::from_millis(750), subscriber.recv()).await {
            Ok(Ok(ev)) => panic!(
                "nothing may publish while the journal table is hidden; got seq {}",
                ev.seq
            ),
            _ => {
                // Expected: the journal is unreadable, so there is nothing to publish
            }
        }

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

        // A correct tailer publishes NOTHING already committed and only picks up from here.
        // `floor = 3` fails the moment any of seqs 1-3 arrives, which is exactly what the
        // fall-back-to-0 mutation does on its first successful pass.
        let base = probe_until_live(&pool, &mut subscriber, 3).await;
        assert!(
            base > 3,
            "tailer must resume at the recovered high-water mark of 3, not replay from 0 (got {base})"
        );

        tailer_handle.abort();
    }
}
