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
        let mut last_published = match event_journal::high_water_mark(&pool).await {
            Ok(mark) => mark,
            Err(e) => {
                warn!(error = ?e, "failed to fetch initial high-water mark; starting from 0");
                0
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

    #[tokio::test]
    async fn tailer_publishes_committed_rows_in_seq_order() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Start the tailer first (high-water mark is 0)
        let (tx, _rx) = broadcast::channel(64);
        let tailer_handle = spawn(pool.clone(), tx.clone());

        // Subscribe to the broadcast channel BEFORE committing rows
        let mut subscriber = tx.subscribe();

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

        // Give the tailer time to poll and publish the 3 rows
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Collect the published events
        let mut seqs = vec![];
        for _ in 0..3 {
            match tokio::time::timeout(std::time::Duration::from_millis(100), subscriber.recv())
                .await
            {
                Ok(Ok(ev)) => seqs.push(ev.seq),
                _ => break,
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

        // Give the tailer time to poll (should find nothing because the row was rolled back)
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify no events were published
        match tokio::time::timeout(std::time::Duration::from_millis(50), subscriber.recv()).await {
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

        // Give the tailer time to publish the committed row
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Collect the published event
        match tokio::time::timeout(std::time::Duration::from_millis(100), subscriber.recv()).await {
            Ok(Ok(ev)) => {
                // Should only receive seq 1 (the committed row, not the rolled-back one)
                assert_eq!(
                    ev.seq, 1,
                    "should only receive the committed row, not the rolled-back one"
                );
            }
            _ => panic!("expected to receive the committed row"),
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

        // Give the tailer time to poll and publish the 3 rows
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Collect the published events from the first pass
        for _ in 0..3 {
            match tokio::time::timeout(std::time::Duration::from_millis(100), subscriber.recv())
                .await
            {
                Ok(Ok(_)) => {
                    // Expected
                }
                _ => break,
            }
        }

        // Wait for another poll interval (tailer should find no new events)
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify the second pass publishes nothing
        match tokio::time::timeout(std::time::Duration::from_millis(50), subscriber.recv()).await {
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

        // Give the tailer time to poll (it will find nothing from (3, 3])
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Verify no events were published yet (tailer started at the high-water mark)
        match tokio::time::timeout(std::time::Duration::from_millis(50), subscriber.recv()).await {
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

        // Give the tailer time to poll and publish the 2 new rows
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Collect the published events
        let mut seqs = vec![];
        for _ in 0..2 {
            match tokio::time::timeout(std::time::Duration::from_millis(100), subscriber2.recv())
                .await
            {
                Ok(Ok(ev)) => seqs.push(ev.seq),
                _ => break,
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

        // Give the tailer time to publish the row (at least 2 poll intervals)
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Drain the event
        match tokio::time::timeout(std::time::Duration::from_millis(200), subscriber.recv()).await {
            Ok(Ok(_)) => {
                // Expected
            }
            _ => panic!("expected to receive the row"),
        }

        // Close the pool to force a read error on the next poll
        let pool_closed = pool.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            pool_closed.close().await;
        });

        // Wait for the pool to close and the tailer to attempt a read (and fail)
        // The tailer should log the error and continue the loop rather than panicking
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // The tailer should still be running (not aborted by the read error)
        assert!(
            !tailer_handle.is_finished(),
            "tailer should survive the read error"
        );

        tailer_handle.abort();
    }
}
