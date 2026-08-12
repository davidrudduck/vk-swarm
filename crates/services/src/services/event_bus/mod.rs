//! Event bus for durable event streaming.
//!
//! The EventBus provides a replay-to-live streaming model:
//! - Subscribers start at a cursor position in the event journal
//! - The stream replays all journaled events from that cursor
//! - When caught up, the stream switches to live delivery via the broadcast channel
//! - If live delivery lags too far (broadcast buffer overrun), it refills from the journal
//!
//! A background journal tailer (task 013) polls the journal periodically and publishes
//! committed events to the broadcast channel for immediate delivery to live subscribers.

mod tailer;

use db::models::event::SequencedEvent;
use db::models::event_journal::{self, EventJournalError};
use futures::stream::{BoxStream, unfold};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Error type for EventBus operations.
#[derive(Debug, Error)]
pub enum EventBusError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
}

/// Internal state for the replay-to-live stream.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum StreamState {
    Initializing,
    ReplayingJournal {
        events: Vec<SequencedEvent>,
        index: usize,
    },
    Live,
    Closed,
}

/// Subscription state holding all mutable parts.
struct SubscriptionState {
    pool: SqlitePool,
    sender: broadcast::Sender<SequencedEvent>,
    state: StreamState,
    last: i64,
    rx: Option<broadcast::Receiver<SequencedEvent>>,
}

/// The EventBus holds a broadcast sender and provides replay-to-live streaming.
/// It also owns the journal tailer task via a shared Arc so multiple clones can safely coexist.
pub struct EventBus {
    pool: SqlitePool,
    sender: broadcast::Sender<SequencedEvent>,
    tailer_handle: std::sync::Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            sender: self.sender.clone(),
            tailer_handle: self.tailer_handle.clone(),
        }
    }
}

impl EventBus {
    /// Creates a new EventBus and spawns the journal tailer.
    ///
    /// The tailer runs in the background, polling the journal for new events and
    /// publishing them to the broadcast channel. The task will continue running until
    /// explicitly stopped via [`shutdown()`](#method.shutdown).
    pub fn new(pool: SqlitePool, broadcast_capacity: usize) -> Self {
        let (_tx, _rx) = broadcast::channel(broadcast_capacity);
        let tailer = tailer::spawn(pool.clone(), _tx.clone());
        Self {
            pool,
            sender: _tx,
            tailer_handle: std::sync::Arc::new(tokio::sync::Mutex::new(Some(tailer))),
        }
    }

    /// Stops the background tailer task cleanly.
    ///
    /// Locks the tailer handle, takes ownership, and aborts it. Safe to call multiple times
    /// (subsequent calls are no-ops because `take()` makes it idempotent). Abort is not synchronous,
    /// so the task may still be running briefly after this call returns.
    ///
    /// **WARNING**: All clones of this EventBus share the same tailer handle. If one clone calls
    /// `shutdown()`, the tailer stops for ALL clones. Other clones' `subscribe_from()` streams
    /// will park in the Live state forever waiting for broadcast messages that will never arrive
    /// (no error is raised).
    pub async fn shutdown(&self) {
        let mut handle_guard = self.tailer_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
    }

    /// Returns a sender that can publish events to the bus.
    ///
    /// Only the tailer (task 013) should use this to publish events from the journal.
    pub fn sender(&self) -> broadcast::Sender<SequencedEvent> {
        self.sender.clone()
    }

    /// Subscribes to events starting from a given cursor position.
    ///
    /// The stream first replays all events with `seq > cursor` from the journal in ascending order.
    /// Once caught up, it switches to live delivery via the broadcast channel.
    /// If live delivery lags and overruns, it refills from the journal and resumes live delivery.
    ///
    /// Returns: A boxed stream of sequenced events.
    /// Errors: If the journal read fails or the initial high-water mark fails.
    pub fn subscribe_from(
        &self,
        cursor: i64,
    ) -> Result<BoxStream<'static, Result<SequencedEvent, EventBusError>>, EventBusError> {
        let pool = self.pool.clone();
        let sender = self.sender.clone();

        let initial_state = SubscriptionState {
            pool,
            sender,
            state: StreamState::Initializing,
            last: cursor,
            rx: None,
        };

        Ok(Box::pin(unfold(initial_state, |mut state| async move {
            loop {
                match &mut state.state {
                    StreamState::Initializing => {
                        // Subscribe before reading journal (critical invariant 1)
                        let rx = state.sender.subscribe();
                        state.rx = Some(rx);

                        // Read fresh high-water mark (critical invariant 3)
                        let mark = match event_journal::high_water_mark(&state.pool).await {
                            Ok(m) => m,
                            Err(e) => {
                                return Some((Err(EventBusError::Journal(e)), state));
                            }
                        };

                        // Read replay events
                        match event_journal::read_range(&state.pool, state.last, mark).await {
                            Ok(events) => {
                                state.state = StreamState::ReplayingJournal { events, index: 0 };
                                continue;
                            }
                            Err(e) => {
                                return Some((Err(EventBusError::Journal(e)), state));
                            }
                        }
                    }

                    StreamState::ReplayingJournal { events, index } => {
                        if *index < events.len() {
                            let seq_ev = events[*index].clone();
                            *index += 1;
                            state.last = seq_ev.seq;
                            return Some((Ok(seq_ev), state));
                        } else {
                            // Done replaying; switch to live
                            state.state = StreamState::Live;
                            continue;
                        }
                    }

                    StreamState::Live => {
                        if let Some(rx) = &mut state.rx {
                            match rx.recv().await {
                                Ok(ev) => {
                                    // Tolerate duplicates (critical invariant 2)
                                    if ev.seq > state.last {
                                        state.last = ev.seq;
                                        return Some((Ok(ev), state));
                                    }
                                    // Skip duplicate and continue polling
                                    continue;
                                }
                                Err(broadcast::error::RecvError::Lagged(_)) => {
                                    // Overrun; refill from journal (critical invariant 4)
                                    let mark = match event_journal::high_water_mark(&state.pool)
                                        .await
                                    {
                                        Ok(m) => m,
                                        Err(e) => {
                                            return Some((Err(EventBusError::Journal(e)), state));
                                        }
                                    };

                                    match event_journal::read_range(&state.pool, state.last, mark)
                                        .await
                                    {
                                        Ok(events) => {
                                            state.state =
                                                StreamState::ReplayingJournal { events, index: 0 };
                                            continue;
                                        }
                                        Err(e) => {
                                            return Some((Err(EventBusError::Journal(e)), state));
                                        }
                                    }
                                }
                                Err(broadcast::error::RecvError::Closed) => {
                                    return None;
                                }
                            }
                        } else {
                            return None;
                        }
                    }

                    StreamState::Closed => {
                        return None;
                    }
                }
            }
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::event::NodeEvent;
    use db::test_utils::create_test_pool_with_migrations;
    use futures::StreamExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn subscribe_from_zero_replays_all_then_goes_live() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create and populate the bus
        let bus = EventBus::new(pool.clone(), 64);
        let sender = bus.sender();

        // Journal 3 events
        {
            let mut tx = pool.begin().await.unwrap();
            for _i in 1..=3 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            }
            tx.commit().await.unwrap();
        }

        // Subscribe from 0 and collect replay
        let mut stream = bus.subscribe_from(0).unwrap();
        let mut seqs = vec![];

        // Collect the 3 replayed events
        for _ in 0..3 {
            match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(ev))) => seqs.push(ev.seq),
                _ => break,
            }
        }

        assert_eq!(seqs, vec![1, 2, 3], "should replay seqs 1, 2, 3");

        // Emit a 4th event live and assert it arrives
        sender
            .send(SequencedEvent {
                seq: 4,
                event: NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                },
            })
            .ok();

        match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(ev))) => assert_eq!(ev.seq, 4, "live event should arrive"),
            _ => panic!("live event 4 did not arrive"),
        }
    }

    #[tokio::test]
    async fn subscribe_from_cursor_skips_already_seen() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Journal 5 events
        {
            let mut tx = pool.begin().await.unwrap();
            for _ in 0..5 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            }
            tx.commit().await.unwrap();
        }

        // Subscribe from cursor 3 (skip 1,2,3; get 4,5)
        let bus = EventBus::new(pool.clone(), 64);
        let mut stream = bus.subscribe_from(3).unwrap();

        let mut seqs = vec![];
        for _ in 0..2 {
            match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(ev))) => seqs.push(ev.seq),
                _ => break,
            }
        }

        assert_eq!(seqs, vec![4, 5], "should only get seqs 4 and 5");
    }

    #[tokio::test]
    async fn no_journaled_event_is_skipped_across_the_handoff() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create bus
        let bus = EventBus::new(pool.clone(), 64);
        let sender = bus.sender();

        // Journal 3 events
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

        // Start subscription (creates the replay-to-live handoff)
        let mut stream = bus.subscribe_from(0).unwrap();

        // Emit more events concurrently during the handoff
        let sender_clone = sender.clone();
        let emit_task = tokio::spawn(async move {
            for i in 4..=6 {
                sender_clone
                    .send(SequencedEvent {
                        seq: i,
                        event: NodeEvent::TaskCreated {
                            task_id: Uuid::new_v4(),
                            project_id: Uuid::new_v4(),
                        },
                    })
                    .ok();
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });

        // Collect all events
        let mut seqs = vec![];
        for _ in 0..6 {
            match tokio::time::timeout(std::time::Duration::from_secs(2), stream.next()).await {
                Ok(Some(Ok(ev))) => seqs.push(ev.seq),
                _ => break,
            }
        }

        emit_task.await.ok();

        // All seqs 1-6 should be present (may have duplicates)
        let unique_seqs: std::collections::HashSet<_> = seqs.iter().cloned().collect();
        for seq in 1..=6 {
            assert!(
                unique_seqs.contains(&seq),
                "seq {} is missing from the stream",
                seq
            );
        }
    }

    #[tokio::test]
    async fn duplicates_are_tolerated_not_errors() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Journal 1 event
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        let bus = EventBus::new(pool.clone(), 64);
        let sender = bus.sender();
        let mut stream = bus.subscribe_from(0).unwrap();

        // Collect the replayed event (seq 1)
        match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(ev))) => assert_eq!(ev.seq, 1),
            _ => panic!("failed to get seq 1"),
        }

        // Now emit the same seq live (a duplicate from the handoff race)
        sender
            .send(SequencedEvent {
                seq: 1,
                event: NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                },
            })
            .ok();

        // The duplicate should be tolerated and skipped
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Emit a new event
        sender
            .send(SequencedEvent {
                seq: 2,
                event: NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                },
            })
            .ok();

        // Collect seq 2 (the duplicate 1 should have been skipped)
        match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(ev))) => assert_eq!(ev.seq, 2, "stream should skip duplicate and continue"),
            _ => panic!("seq 2 did not arrive"),
        }
    }

    #[tokio::test]
    async fn lagged_refills_from_journal_and_resumes_live() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create bus with TINY capacity to force Lagged
        let bus = EventBus::new(pool.clone(), 2);
        let sender = bus.sender();

        // Journal 5 events
        {
            let mut tx = pool.begin().await.unwrap();
            for _ in 0..5 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            }
            tx.commit().await.unwrap();
        }

        let mut stream = bus.subscribe_from(0).unwrap();

        // Collect the 5 replayed events
        let mut seen_seqs = vec![];
        for _ in 0..5 {
            match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(ev))) => seen_seqs.push(ev.seq),
                _ => break,
            }
        }
        assert_eq!(seen_seqs, vec![1, 2, 3, 4, 5]);

        // Now journal events 6-10 and emit them on the broadcast channel
        // to overrun the tiny buffer and force Lagged
        {
            let mut tx = pool.begin().await.unwrap();
            for _i in 6..=10 {
                let event = NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                };
                let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            }
            tx.commit().await.unwrap();
        }

        // Emit the journaled events on the broadcast channel
        for i in 6..=10 {
            sender
                .send(SequencedEvent {
                    seq: i,
                    event: NodeEvent::TaskCreated {
                        task_id: Uuid::new_v4(),
                        project_id: Uuid::new_v4(),
                    },
                })
                .ok();
        }

        // Give the refill a chance to happen
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // The stream should recover via refill and continue with live events
        let mut post_lagged_seqs = vec![];
        for _ in 0..5 {
            match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(ev))) => post_lagged_seqs.push(ev.seq),
                _ => break,
            }
        }

        // Should have seqs 6-10 from the refill or live delivery
        let unique_post: std::collections::HashSet<_> = post_lagged_seqs.iter().cloned().collect();
        for seq in 6..=10 {
            assert!(
                unique_post.contains(&seq),
                "seq {} should arrive after Lagged refill",
                seq
            );
        }

        // Emit a new event and verify it arrives (stream is still live)
        sender
            .send(SequencedEvent {
                seq: 11,
                event: NodeEvent::TaskCreated {
                    task_id: Uuid::new_v4(),
                    project_id: Uuid::new_v4(),
                },
            })
            .ok();

        match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(ev))) => assert_eq!(
                ev.seq, 11,
                "stream should resume live delivery after Lagged refill"
            ),
            _ => panic!("seq 11 did not arrive after Lagged recovery"),
        }
    }

    #[tokio::test]
    async fn first_occurrences_are_ascending() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 16);
        let sender = bus.sender();

        // Journal 3 events
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

        let mut stream = bus.subscribe_from(0).unwrap();

        // Emit live events during replay
        let sender_clone = sender.clone();
        let emit_task = tokio::spawn(async move {
            for i in 4..=6 {
                sender_clone
                    .send(SequencedEvent {
                        seq: i,
                        event: NodeEvent::TaskCreated {
                            task_id: Uuid::new_v4(),
                            project_id: Uuid::new_v4(),
                        },
                    })
                    .ok();
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        });

        // Collect all events and track first occurrence of each seq
        let mut first_occurrences = std::collections::HashMap::new();
        let mut count = 0;

        loop {
            if count >= 9 {
                break;
            }
            match tokio::time::timeout(std::time::Duration::from_secs(2), stream.next()).await {
                Ok(Some(Ok(ev))) => {
                    first_occurrences.entry(ev.seq).or_insert(count);
                    count += 1;
                }
                _ => break,
            }
        }

        emit_task.await.ok();

        // Verify first occurrences are ascending
        let mut seqs: Vec<_> = first_occurrences.keys().copied().collect();
        seqs.sort();
        let occurrence_order: Vec<_> = seqs.iter().map(|&s| first_occurrences[&s]).collect();

        for i in 1..occurrence_order.len() {
            assert!(
                occurrence_order[i - 1] < occurrence_order[i],
                "seq {} should occur before seq {} in the stream",
                seqs[i - 1],
                seqs[i]
            );
        }
    }

    #[tokio::test]
    async fn initial_read_error_surfaces_to_the_consumer() {
        // Create a pool, then close it to simulate a failed pool
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        pool.close().await;

        // Try to subscribe — the first journal read should fail
        let bus = EventBus::new(pool, 64);
        let mut stream = bus.subscribe_from(0).unwrap();

        // The first item should be an error
        match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
            Ok(Some(Err(_))) => {
                // Expected: error surfaces
            }
            Ok(Some(Ok(_))) => {
                panic!("expected error on closed pool, got event");
            }
            _ => {
                panic!("expected error to surface on closed pool");
            }
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

    /// Blocks until the tailer is provably publishing, by committing probe rows until one comes
    /// back. `tokio::spawn` only schedules the task, so without this the tailer's initial
    /// high-water-mark read can land after the row under test and skip it — silence would then
    /// prove nothing about `shutdown()`.
    ///
    /// Returns only once the NEWEST probe row has been received, so no stale probe event can be
    /// mistaken for a post-shutdown publication.
    async fn wait_until_tailer_publishes(
        pool: &SqlitePool,
        subscriber: &mut broadcast::Receiver<SequencedEvent>,
    ) {
        for _ in 0..10 {
            let seq = commit_one(pool).await;
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                match tokio::time::timeout(remaining, subscriber.recv()).await {
                    Ok(Ok(ev)) if ev.seq == seq => return,
                    Ok(Ok(_)) => continue,
                    _ => break,
                }
            }
        }
        panic!("the tailer never published a probe row, so this test cannot observe shutdown");
    }

    #[tokio::test]
    async fn shutdown_stops_the_tailer() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create an EventBus with a spawned tailer
        let bus = EventBus::new(pool.clone(), 64);

        // Subscribe BEFORE the shutdown-and-commit window. A tokio broadcast receiver never
        // receives history: a subscriber created afterwards cannot observe a still-running
        // tailer's publish, which is what made the previous version of this test vacuous — a
        // literal no-op `shutdown()` passed it.
        let mut subscriber = bus.sender().subscribe();

        // Prove the tailer IS publishing first, so the silence below is attributable to shutdown()
        // and not to a tailer that was never live.
        wait_until_tailer_publishes(&pool, &mut subscriber).await;

        // Shutdown the tailer
        bus.shutdown().await;

        // Commit a journal row AFTER shutdown. A live tailer publishes it within TAIL_INTERVAL.
        let post_shutdown_seq = commit_one(&pool).await;

        // Nothing may arrive. The window is many multiples of TAIL_INTERVAL (75ms) so that a
        // still-running tailer has no plausible excuse for being silent.
        match tokio::time::timeout(std::time::Duration::from_secs(2), subscriber.recv()).await {
            Ok(Ok(ev)) => panic!(
                "tailer should be stopped; it published seq {} after shutdown (expected silence for seq {})",
                ev.seq, post_shutdown_seq
            ),
            _ => {
                // Expected: no events published
            }
        }
    }
}
