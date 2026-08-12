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
            Err(_) => None,      // Timeout (deadline passed)
        }
    }

    #[tokio::test]
    async fn tailer_publishes_committed_rows_in_seq_order() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first (high-water mark is 0)
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel BEFORE committing rows
        let mut subscriber = tx.subscribe();

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

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

        // Verify seqs are 1, 2, 3 in order
        assert_eq!(
            seqs,
            vec![1, 2, 3],
            "tailer should publish seqs 1, 2, 3 in order"
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

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

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
                // Should only receive seq 1 (the committed row, not the rolled-back one)
                assert_eq!(
                    ev.seq, 1,
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

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

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
        for _ in 0..3 {
            match poll_for_event(&mut subscriber, deadline).await {
                Some(_) => {
                    // Expected
                }
                None => break,
            }
        }

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

        // Drop the tailer
        tailer_handle.abort();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Create a new tailer (will start at the current high-water mark of 3)
        let (tx2, _rx2) = broadcast::channel(64);
        let tailer_handle2 = spawn(pool.clone(), tx2.clone());

        // Subscribe to the new tailer
        let mut subscriber2 = tx2.subscribe();

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

        // Only the 2 new rows (seqs 4, 5) should be published
        assert_eq!(
            seqs,
            vec![4, 5],
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

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Append and commit 1 row before the outage
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            tx_write.commit().await.unwrap();
        }

        // Drain the event using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                assert_eq!(ev.seq, 1, "expected to receive seq 1");
            }
            None => panic!("expected to receive the row"),
        }

        // Induce transient failure via table rename (fires high_water_mark, outer arm)
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        // Wait several multiples of TAIL_INTERVAL (75ms * 3 = 225ms) to ensure the tailer
        // has attempted multiple reads and confirmed nothing is published during the outage.
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;

        // Verify no additional event was published during outage (fault fired).
        // Use a bounded wait (3 * TAIL_INTERVAL) to check for spurious events.
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(_)) => panic!("no event should publish during table-hidden outage"),
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

        // The cursor should not have advanced during the error, so the tailer should still be at mark 1
        // Commit a new row and verify it publishes (proving cursor didn't advance past the error)
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            tx_write.commit().await.unwrap();
        }

        // Poll for the recovered row using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                assert_eq!(
                    ev.seq, 2,
                    "should publish seq 2 after recovery; cursor did not advance past error"
                );
            }
            None => panic!("expected to receive seq 2 after recovery from transient error"),
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

        // Give the tailer a moment to start and initialize
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Append and commit 1 row
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            tx_write.commit().await.unwrap();
        }

        // Drain the published event using deadline-based polling
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        match poll_for_event(&mut subscriber, deadline).await {
            Some(ev) => {
                assert_eq!(ev.seq, 1);
            }
            None => panic!("expected to receive seq 1"),
        }

        // Commit row 2 before corrupting it
        {
            let mut tx_write = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx_write, &event).await.unwrap();
            tx_write.commit().await.unwrap();
        }

        // Corrupt payload on seq 2 so read_range fails at serde_json::from_str (inner arm)
        sqlx::query("UPDATE event_journal SET payload = '{not json' WHERE seq = 2")
            .execute(&pool)
            .await
            .unwrap();

        // Wait for the tailer to attempt multiple reads and fail at deserialization
        tokio::time::sleep(std::time::Duration::from_millis(225)).await;

        // Verify nothing was published during the corruption (fault fired).
        // Use bounded wait (3 * TAIL_INTERVAL) to ensure we're checking real state.
        match tokio::time::timeout(std::time::Duration::from_millis(225), subscriber.recv()).await {
            Ok(Ok(_)) => panic!("no event should publish while payload is corrupted"),
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
            sqlx::query("UPDATE event_journal SET payload = ? WHERE seq = 2")
                .bind(payload)
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
                    ev.seq, 2,
                    "should publish seq 2 after repair; if cursor advanced during error, this would be skipped"
                );
            }
            None => panic!("expected to receive seq 2 after recovery from read error"),
        }

        tailer_handle.abort();
    }
}
