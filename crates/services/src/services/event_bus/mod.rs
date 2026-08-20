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

pub use tailer::TailerHealth;

use std::sync::Arc;

use db::models::event::SequencedEvent;
use db::models::event_journal::{self, EventJournalError};
use futures::stream::{BoxStream, unfold};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::error;

/// How long [`EventBus::new`] waits for the tailer's readiness signal before giving up on closing
/// the startup race and returning anyway.
///
/// The tailer's own initial-mark retry loop (`tailer::spawn`) backs off up to 1000ms per attempt
/// and never gives up, so 10s covers roughly 12 attempts of that loop — generous headway over the
/// ~1-in-20 straddle this constant exists to close. A journal still unreadable after 10s at boot is
/// a node with problems well beyond event delivery, and `spawn`'s own retry-forever loop keeps
/// trying in the background regardless of what `new` decides here.
const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Error type for EventBus operations.
#[derive(Debug, Error)]
pub enum EventBusError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
}

/// Internal state for the replay-to-live stream.
#[derive(Debug)]
enum StreamState {
    Initializing,
    ReplayingJournal {
        events: Vec<SequencedEvent>,
        index: usize,
    },
    Live,
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
    tailer_health: Arc<TailerHealth>,
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            sender: self.sender.clone(),
            tailer_handle: self.tailer_handle.clone(),
            tailer_health: self.tailer_health.clone(),
        }
    }
}

impl EventBus {
    /// Creates a new EventBus and spawns the journal tailer.
    ///
    /// The tailer runs in the background, polling the journal for new events and
    /// publishing them to the broadcast channel. The task will continue running until
    /// explicitly stopped via [`shutdown()`](#method.shutdown).
    ///
    /// Awaits the tailer's readiness signal (bounded by [`READY_TIMEOUT`]) before returning. Once
    /// readiness resolves, the tailer's cursor is fixed, so every commit from this point on is
    /// strictly above it and is owed to subscribers: the startup race where a commit lands between
    /// `subscribe_from`'s own high-water read and the tailer's initial one — losing that commit
    /// permanently — becomes unrepresentable. See the decisions-ledger for task 018.
    ///
    /// `spawn`'s initial-mark loop itself is retry-forever and is not bounded by this call — only
    /// the WAIT here is bounded. If the journal is still unreadable after `READY_TIMEOUT`, this
    /// logs loudly and returns the bus anyway, with the tailer continuing to retry in the
    /// background; a row committed before the tailer's cursor is eventually established is
    /// silently but correctly not broadcast, exactly as it is today. That degraded case is
    /// unchanged from before this task; every other case is now race-free.
    pub async fn new(pool: SqlitePool, broadcast_capacity: usize) -> Self {
        Self::new_with_ready_timeout(pool, broadcast_capacity, READY_TIMEOUT).await
    }

    /// Implementation behind [`new`](Self::new), with the readiness timeout as a parameter so tests
    /// can drive the timeout path quickly instead of waiting out the full [`READY_TIMEOUT`].
    async fn new_with_ready_timeout(
        pool: SqlitePool,
        broadcast_capacity: usize,
        ready_timeout: std::time::Duration,
    ) -> Self {
        let (sender, _rx) = broadcast::channel(broadcast_capacity);
        let tailer_health = Arc::new(TailerHealth::default());
        let (tailer, ready) =
            tailer::spawn(pool.clone(), sender.clone(), Arc::clone(&tailer_health));

        match tokio::time::timeout(ready_timeout, ready).await {
            Ok(Ok(())) => {
                // The tailer's cursor is now fixed; the startup race is closed.
            }
            Ok(Err(_)) => {
                // The sender was dropped without signalling — the tailer task ended before it
                // could establish a cursor. Degrade exactly as the timeout case below does.
                error!(
                    "event bus tailer readiness sender dropped without signalling; the tailer may \
                     have ended before establishing its cursor. Proceeding without the startup-race \
                     guarantee; events committed before the tailer's cursor is established may not \
                     be broadcast"
                );
            }
            Err(_) => {
                error!(
                    ready_timeout_secs = ready_timeout.as_secs(),
                    "event bus tailer did not signal readiness within the timeout; its initial \
                     high-water mark is still being retried in the background. Proceeding without \
                     the startup-race guarantee; events committed in this window may not be \
                     broadcast"
                );
            }
        }

        Self {
            pool,
            sender,
            tailer_handle: std::sync::Arc::new(tokio::sync::Mutex::new(Some(tailer))),
            tailer_health,
        }
    }

    /// The tailer's liveness counters, shared across all clones of this `EventBus` exactly as
    /// `tailer_handle` is. A caller can use this to expose a health surface that does not depend
    /// on inferring liveness from timing side-effects on the broadcast channel — see the
    /// decisions-ledger for task 016.
    pub fn tailer_health(&self) -> &TailerHealth {
        &self.tailer_health
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
                                    // Overrun; refill from journal (critical invariant 4).
                                    //
                                    // On a refill FAILURE the state resets to Initializing before
                                    // the Err is yielded: staying Live would let a consumer that
                                    // polls past the Err silently skip the lagged-out range (the
                                    // receiver has already been repositioned past it). From
                                    // Initializing the next poll re-replays from state.last, so
                                    // the stream self-heals with no gap.
                                    let mark = match event_journal::high_water_mark(&state.pool)
                                        .await
                                    {
                                        Ok(m) => m,
                                        Err(e) => {
                                            state.state = StreamState::Initializing;
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
                                            state.state = StreamState::Initializing;
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
    use std::sync::atomic::Ordering;
    use uuid::Uuid;

    #[tokio::test]
    async fn subscribe_from_zero_replays_all_then_goes_live() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create and populate the bus
        let bus = EventBus::new(pool.clone(), 64).await;
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
        let bus = EventBus::new(pool.clone(), 64).await;
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
        let bus = EventBus::new(pool.clone(), 64).await;
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

        let bus = EventBus::new(pool.clone(), 64).await;
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
        let bus = EventBus::new(pool.clone(), 2).await;
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

        let bus = EventBus::new(pool.clone(), 16).await;
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

        // Try to subscribe — the first journal read should fail.
        //
        // UNDICTATED CHOICE (task 018): drives `new_with_ready_timeout` with a short timeout
        // rather than the public `new`. A closed pool makes the tailer's initial `high_water_mark`
        // fail immediately and forever (see task 013's retry-forever loop), so its readiness never
        // fires; with the public `new`'s full 10s `READY_TIMEOUT` this test would now cost 10s for
        // no additional coverage of the property under test (that `subscribe_from` surfaces a
        // journal read error). A short timeout keeps it fast without touching what it asserts.
        let bus =
            EventBus::new_with_ready_timeout(pool, 64, std::time::Duration::from_millis(50)).await;
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

    /// **REQUIRED (task 018).** `EventBus::new` must RETURN even when the tailer's readiness never
    /// fires, rather than hang forever. `tailer::spawn`'s initial-mark loop is retry-forever by
    /// design and stays that way (task 013, panels 5/6) — see the decisions-ledger for task 018's
    /// withdrawn section 2 — so `new` bounds its own WAIT via `tokio::time::timeout` instead of
    /// bounding the tailer's retry loop.
    ///
    /// Drives `new_with_ready_timeout` directly with a short timeout so this test does not have to
    /// wait out the full 10s `READY_TIMEOUT`: the mechanism under test is the `tokio::time::timeout`
    /// wrapper around the readiness receiver, which behaves identically at any duration, so a short
    /// one proves the same thing faster.
    ///
    /// The table is renamed away BEFORE the bus is constructed, so the tailer's very first
    /// `high_water_mark` call fails and its retry-forever loop cannot resolve within the short
    /// timeout — the same fault
    /// `tailer_retries_the_initial_high_water_mark_instead_of_falling_back_to_zero` (`tailer.rs`)
    /// holds open, which `new` must survive without hanging rather than by falling back to a
    /// fabricated cursor (that design was tried and withdrawn — see the ledger).
    ///
    /// Mutation proof (task 018's REQUIRED bar): make `new_with_ready_timeout` `.await` the
    /// readiness receiver unconditionally, with no `tokio::time::timeout` wrapper — this test must
    /// then hang and fail, not pass.
    #[tokio::test]
    async fn new_returns_even_if_the_tailer_never_signals_readiness() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Hide the table BEFORE constructing the bus, so the tailer's initial high_water_mark call
        // fails immediately and its retry-forever loop never resolves.
        sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_hidden")
            .execute(&pool)
            .await
            .unwrap();

        let short_timeout = std::time::Duration::from_millis(200);

        // An outer deadline many times the configured timeout: a safety net so a driver that
        // ignores `ready_timeout` and awaits unconditionally fails this test with a diagnosis
        // instead of hanging the whole suite.
        let start = tokio::time::Instant::now();
        let bus = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            EventBus::new_with_ready_timeout(pool.clone(), 64, short_timeout),
        )
        .await
        .expect(
            "EventBus::new_with_ready_timeout did not return within 5s (25x its configured 200ms \
             readiness timeout); it is awaiting the tailer's readiness signal unconditionally \
             instead of bounding the wait",
        );
        let elapsed = start.elapsed();

        assert!(
            elapsed >= short_timeout,
            "returned after {elapsed:?}, before the configured {short_timeout:?} readiness timeout \
             even elapsed; the journal table is hidden and the tailer cannot have signalled \
             readiness that fast"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "returned after {elapsed:?}, far longer than the configured {short_timeout:?} \
             readiness timeout; `new_with_ready_timeout` is not bounding its wait by the timeout \
             it was given"
        );

        // Repair, so the tailer — still retrying in the background per its own retry-forever
        // contract — can eventually establish a real cursor, and no orphaned poll keeps failing
        // against a renamed table for the rest of the test process.
        sqlx::query("ALTER TABLE event_journal_hidden RENAME TO event_journal")
            .execute(&pool)
            .await
            .unwrap();

        bus.shutdown().await;
    }

    /// Upper bound on how long [`EventBus::new`] may take on a HEALTHY pool, where the tailer's
    /// very first `high_water_mark` read succeeds and readiness fires immediately.
    ///
    /// **Derived from measurement, not chosen.** `EventBus::new` was instrumented and the elapsed
    /// time sampled inside the FULL parallel `cargo test -p services --lib` run (not in isolation,
    /// so the sample carries the suite's own contention) on a 4-core box: **12 runs on a quiet
    /// machine (`pgrep -x cargo` empty) and 4 more under a deliberate 6-way CPU load — 16 samples,
    /// every one of them 1ms or 2ms, max 2ms.** Under load the whole suite stretched from ~12.2s
    /// to ~14.9s while this construction did not move, which is the expected shape: it is one
    /// SQLite `MAX(seq)` read on a freshly migrated database, not a scheduling-sensitive wait.
    ///
    /// The bound is [`READY_TIMEOUT`] / 10, so it is stated relative to the constant it guards and
    /// survives a future change to it. At the current 10s that is 1000ms:
    ///
    /// - **500x the observed maximum**, which is the headroom over scheduling noise — this must
    ///   never be the thing that decides the verdict on a loaded CI box.
    /// - **10x below `READY_TIMEOUT`**, which is what gives it teeth: a driver that stops
    ///   observing readiness and waits out the whole budget overshoots this by an order of
    ///   magnitude and cannot creep past it.
    const HEALTHY_READY_BOUND: std::time::Duration =
        std::time::Duration::from_millis((READY_TIMEOUT.as_millis() / 10) as u64);

    /// **REQUIRED (task 018 attempt 2, panel 12's F3).** `EventBus::new` must return because the
    /// tailer's readiness FIRED, not merely because its own timeout eventually expired.
    ///
    /// `new_returns_even_if_the_tailer_never_signals_readiness` above pins only "returns within
    /// budget". Panel 12 proved the gap by replacing the awaited future with `pending()` — never
    /// observe readiness, always sleep the full budget — and the whole lib suite stayed green
    /// while its runtime went from 12.51s to 44.64s. In production every `EventBus::new()` would
    /// then silently cost the full [`READY_TIMEOUT`] and every health surface would still read
    /// green: the same green-while-degraded class task 016 exists to close, reappearing one layer
    /// up.
    ///
    /// This closes it on a healthy pool, where readiness resolves on the tailer's first
    /// `high_water_mark` read, by bounding the construction far below [`READY_TIMEOUT`]. It drives
    /// the PUBLIC `new` deliberately, not `new_with_ready_timeout`: the cost this pins is the real
    /// constant being burned on every construction, and only the public constructor carries it.
    ///
    /// Mutation proof (task 018 attempt 2's REQUIRED bar): replace the awaited `ready` with
    /// `std::future::pending()` — this test must then FAIL, having burned the full
    /// [`READY_TIMEOUT`].
    #[tokio::test]
    async fn new_returns_as_soon_as_a_healthy_tailer_signals_readiness() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let start = tokio::time::Instant::now();
        let bus = EventBus::new(pool.clone(), 64).await;
        let elapsed = start.elapsed();

        assert!(
            elapsed < HEALTHY_READY_BOUND,
            "EventBus::new took {elapsed:?} on a healthy pool, at or past the \
             {HEALTHY_READY_BOUND:?} bound. On a readable journal the tailer signals readiness on \
             its first high_water_mark read, so construction is a matter of milliseconds; anything \
             approaching READY_TIMEOUT ({READY_TIMEOUT:?}) means the readiness signal is not being \
             observed at all and every construction silently waits out the whole budget"
        );

        bus.shutdown().await;
    }

    /// Commits one journal row and returns the seq it was assigned plus the `task_id` that
    /// identifies its body.
    async fn commit_one(pool: &SqlitePool) -> (i64, Uuid) {
        let mut tx = pool.begin().await.unwrap();
        let task_id = Uuid::new_v4();
        let event = NodeEvent::TaskCreated {
            task_id,
            project_id: Uuid::new_v4(),
        };
        let seq = event_journal::append(&mut *tx, &event).await.unwrap();
        tx.commit().await.unwrap();
        (seq, task_id)
    }

    /// Commits ONE row and asserts the TAILER publishes it — at its exact seq, carrying the body
    /// that was committed. Returns that seq.
    ///
    /// **This replaces `wait_until_tailer_publishes`, a 10-attempt probe-commit retry loop deleted
    /// in task 018 attempt 2 (panel 12's F2).** That helper's entire stated justification was that
    /// "`tokio::spawn` only schedules the task, so without this the tailer's initial
    /// high-water-mark read can land after the row under test and skip it". Task 018 made
    /// `EventBus::new` AWAIT the tailer's readiness signal before returning, which falsified that
    /// premise: by the time any caller here runs, the tailer's initial cursor is already fixed, so
    /// every subsequent commit is strictly above it and is unconditionally owed to subscribers.
    /// That is precisely the edge `tailer.rs`'s `await_ready` documents — "a happens-before edge
    /// that costs no journal row, so a row committed after it is unconditionally owed to
    /// subscribers and its seq can be asserted ABSOLUTELY" — and
    /// `a_row_committed_after_readiness_is_never_dropped` (`tailer.rs`) is this same shape one
    /// layer down, against `tailer::spawn` directly.
    ///
    /// Keeping a retry loop here would also have kept alive, in the lib suite, exactly the pattern
    /// task 018 deleted from the end-to-end suite: probe-relative liveness rebases the frame on
    /// each attempt, so a tailer that DROPPED a row simply moves the goalposts and still satisfies
    /// the precondition. The wait below is a single strict, exact-seq observation instead: the
    /// next thing this subscriber receives must be the committed row, not merely something.
    ///
    /// This is the ONLY place in this module that observes the TAILER's output — every other test
    /// here drives `sender` by hand to exercise `subscribe_from`, and is deliberately
    /// tailer-independent. So it is the only place a payload assertion belongs: matching the row's
    /// body as well as its seq means a tailer publishing fabricated payloads cannot satisfy the
    /// liveness precondition this helper exists to establish.
    async fn expect_tailer_publishes_a_committed_row(
        pool: &SqlitePool,
        subscriber: &mut broadcast::Receiver<SequencedEvent>,
    ) -> i64 {
        let (seq, task_id) = commit_one(pool).await;

        // Generous deadline: a safety net so a dead tailer fails with a diagnosis rather than
        // hanging the suite, never the thing that decides the verdict. Matches the 30s the other
        // tailer-observing waits in this module and in `tailer.rs` already use.
        match tokio::time::timeout(std::time::Duration::from_secs(30), subscriber.recv()).await {
            Ok(Ok(ev)) if ev.seq == seq => {
                match ev.event {
                    NodeEvent::TaskCreated {
                        task_id: delivered, ..
                    } => assert_eq!(
                        delivered, task_id,
                        "the tailer published seq {seq} carrying a body that is not the one \
                         committed at that seq"
                    ),
                    ref other => panic!(
                        "the tailer published {other:?} at seq {seq}; the committed row is a \
                         TaskCreated"
                    ),
                }
                seq
            }
            Ok(Ok(other)) => panic!(
                "the tailer published seq {} first; seq {seq} was the only row owed to this \
                 subscriber, which was created before that row was committed and after \
                 `EventBus::new` had already awaited the tailer's readiness",
                other.seq
            ),
            Ok(Err(e)) => panic!(
                "the broadcast receiver failed before seq {seq} arrived: {e:?}; the tailer cannot \
                 be observed, so nothing downstream of this precondition proves anything"
            ),
            Err(_) => panic!(
                "the tailer published nothing within 30s; seq {seq} was committed after \
                 `EventBus::new` awaited readiness and cannot legitimately be skipped"
            ),
        }
    }

    /// Consumes anything still queued on `subscriber`, returning once the channel has stayed silent
    /// for one quiet period (or a hard cap elapses).
    ///
    /// `expect_tailer_publishes_a_committed_row` returns the instant it sees ONE copy of its row,
    /// and `shutdown_stops_the_tailer` needs an EMPTY pipe afterwards so no stale event can be
    /// mistaken for a post-shutdown publication. That holds on its own only while the bus publishes
    /// each row exactly once. A bus that published a row TWICE leaves the second copy buffered, and
    /// `shutdown_stops_the_tailer` — which panics on any event at all after `shutdown()` — would
    /// then go red for a reason that has nothing to do with shutdown, non-deterministically,
    /// depending on whether the second publisher got its tick in before the abort. Duplicate
    /// publication is the subject of `the_bus_publishes_a_committed_row_exactly_once` and belongs
    /// to that test alone; this keeps it from leaking sideways into an unrelated assertion.
    async fn drain_until_quiet(subscriber: &mut broadcast::Receiver<SequencedEvent>) {
        let cap = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < cap {
            match tokio::time::timeout(std::time::Duration::from_millis(250), subscriber.recv())
                .await
            {
                Ok(Ok(_)) => continue,
                _ => return,
            }
        }
    }

    /// `broadcast_capacity` must actually size the channel this bus publishes into.
    ///
    /// Nothing asserted it. `lagged_refills_from_journal_and_resumes_live` is the only test that
    /// cares about the buffer size, and it passes whether or not `Lagged` ever fires: with a large
    /// buffer its events simply arrive live, every seq expectation still holds, and the refill arm
    /// it is named for is never entered. A constructor that ignored the argument and used a fixed
    /// 1024-slot channel passed all 263 tests — so `subscribe_from`'s Lagged/refill arm had no live
    /// coverage at all, and a caller could not bound the bus's memory.
    ///
    /// This asserts the capacity directly, at the only place it is observable: a capacity-2 channel
    /// must drop exactly the one oldest event once a third is queued behind an unpolled receiver.
    /// It is deterministic, not timing-dependent — the sends and the `try_recv` are synchronous, and
    /// the journal is empty so the tailer contributes nothing to the channel and cannot perturb the
    /// count.
    ///
    /// **This closes a hole in task 005, which is already marked passed.** `mod.rs` is in task 013's
    /// file set, so it is fixed here rather than by reopening 005; see the decisions-ledger.
    #[tokio::test]
    async fn new_honours_the_requested_broadcast_capacity() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 2).await;
        let sender = bus.sender();
        let mut rx = sender.subscribe();

        assert_eq!(
            event_journal::high_water_mark(&pool).await.unwrap(),
            0,
            "the journal must be empty, or the tailer could add events and skew the Lagged count"
        );

        for seq in 1..=3 {
            sender
                .send(SequencedEvent {
                    seq,
                    event: NodeEvent::TaskCreated {
                        task_id: Uuid::new_v4(),
                        project_id: Uuid::new_v4(),
                    },
                })
                .ok();
        }

        let observed = rx.try_recv();
        bus.shutdown().await;

        match observed {
            Err(broadcast::error::TryRecvError::Lagged(n)) => assert_eq!(
                n, 1,
                "a capacity-2 bus must drop exactly the one oldest event when three are queued"
            ),
            other => panic!(
                "EventBus::new ignored the requested capacity of 2: three queued events did not \
                 overrun the buffer (got {other:?}). A bus that silently uses a larger buffer makes \
                 subscribe_from's Lagged/refill arm unreachable even for a caller that asks for a \
                 tiny buffer precisely to provoke it."
            ),
        }
    }

    /// The bus must publish each committed journal row EXACTLY ONCE.
    ///
    /// This is task 013's property 4 — "one tailer per DBService" — stated as something observable
    /// instead of as a shape the code happens to have. Nothing stated it before. Every other test
    /// in this module drives `sender` by hand with fabricated `SequencedEvent`s and is deliberately
    /// tailer-independent, and every test in `tailer.rs` calls `tailer::spawn` directly and never
    /// touches `EventBus::new` at all — so a constructor that spawned a SECOND tailer on the same
    /// channel (with `Clone` still correct and `shutdown()` still aborting both handles, so shutdown
    /// semantics stayed intact) passed the entire 267-test suite three times over, while a scratch
    /// probe showed the bus delivering one committed row twice.
    ///
    /// It is a real defect rather than harmless waste: two tailers double the journal polling
    /// against the same SQLite pool, halve the effective broadcast buffer — which pushes
    /// `subscribe_from` into its `Lagged`/refill path far sooner, at two extra queries each time —
    /// and hand true duplicates to any direct `sender().subscribe()` consumer, which is exactly what
    /// task 014's startup wiring is the natural place to become.
    ///
    /// `bus.sender().subscribe()` is the required instrument. `subscribe_from` is the wrong one:
    /// its Live arm dedups on `ev.seq > last`, so it would swallow precisely the evidence.
    #[tokio::test]
    async fn the_bus_publishes_a_committed_row_exactly_once() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Drives EventBus::new, NOT tailer::spawn — that is the whole point. The tailer this
        // observes is the one the constructor wired up, which is the only thing that can be wrong
        // in the way this test exists to catch.
        let bus = EventBus::new(pool.clone(), 64).await;
        let mut subscriber = bus.sender().subscribe();

        // `EventBus::new` AWAITS the tailer's readiness signal before returning (task 018), so the
        // tailer's initial cursor is already fixed at this point and the row committed below is
        // strictly above it — unconditionally owed to this subscriber, which was created first.
        // No probe loop is needed to buy that happens-before edge, and task 018 attempt 2 deleted
        // the one that used to sit here (panel 12's F2): its stated justification was that `new`
        // DROPPED the readiness receiver, which this task falsified.
        //
        // The journal is asserted empty so the counted seq is an ABSOLUTE 1 rather than whatever a
        // probe loop happened to leave behind — the same device
        // `a_row_committed_after_readiness_is_never_dropped` (`tailer.rs`) uses for the same
        // reason.
        assert_eq!(
            event_journal::high_water_mark(&pool).await.unwrap(),
            0,
            "this test's absolute seq assertion requires a fresh journal"
        );

        let (seq, task_id) = commit_one(&pool).await;
        assert_eq!(seq, 1, "the first row of a fresh journal must be seq 1");

        // First delivery, on a generous deadline: what makes this slow is machine load, never
        // correctness, so the budget is deadline-based rather than a fixed sleep.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut copies = 0usize;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, subscriber.recv()).await {
                Ok(Ok(ev)) if ev.seq == seq => {
                    match ev.event {
                        NodeEvent::TaskCreated {
                            task_id: delivered, ..
                        } => assert_eq!(
                            delivered, task_id,
                            "the bus published seq {seq} carrying a body that is not the one \
                             committed at that seq"
                        ),
                        ref other => panic!(
                            "the bus published {other:?} at seq {seq}; the committed row is a \
                             TaskCreated"
                        ),
                    }
                    copies += 1;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert_eq!(
            copies, 1,
            "the bus published nothing for seq {seq} within 30s; a duplicate-detection test is \
             vacuous unless the row arrives at least once"
        );

        // Then a bounded FURTHER window for a second copy. A second tailer polls on its own
        // independent TAIL_INTERVAL (75ms) tick, so its copy lands within roughly one interval of
        // the first; 2000ms is ~26 intervals and needs no luck. Only copies of THIS seq are
        // counted, which is now belt-and-braces rather than load-bearing: seq 1 is the only row
        // this test ever commits, so there is nothing else on the channel to miscount. It was
        // load-bearing before task 018 attempt 2, when a probe loop left earlier rows behind.
        let second_window = tokio::time::Instant::now() + std::time::Duration::from_millis(2000);
        loop {
            let remaining = second_window.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, subscriber.recv()).await {
                Ok(Ok(ev)) if ev.seq == seq => copies += 1,
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }

        bus.shutdown().await;

        assert_eq!(
            copies, 1,
            "the bus delivered the single committed row at seq {seq} {copies} time(s); \
             `EventBus::new` must spawn exactly ONE tailer (task 013 property 4). A second tailer \
             on the same channel doubles journal polling against one pool, halves the effective \
             broadcast buffer, and gives every direct `sender().subscribe()` consumer true \
             duplicates."
        );
    }

    #[tokio::test]
    async fn shutdown_stops_the_tailer() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create an EventBus with a spawned tailer
        let bus = EventBus::new(pool.clone(), 64).await;

        // Subscribe BEFORE the shutdown-and-commit window. A tokio broadcast receiver never
        // receives history: a subscriber created afterwards cannot observe a still-running
        // tailer's publish, which is what made the previous version of this test vacuous — a
        // literal no-op `shutdown()` passed it.
        let mut subscriber = bus.sender().subscribe();

        // Prove the tailer IS publishing first, so the silence below is attributable to shutdown()
        // and not to a tailer that was never live. Exact-seq and body-checked, not probe-relative:
        // `EventBus::new` has already awaited readiness, so this row cannot legitimately be
        // skipped (task 018 attempt 2 — see `expect_tailer_publishes_a_committed_row`).
        expect_tailer_publishes_a_committed_row(&pool, &mut subscriber).await;

        // Return to an EMPTY pipe before shutting down, so a buffered second copy of the liveness
        // row cannot be mistaken below for a post-shutdown publication.
        drain_until_quiet(&mut subscriber).await;

        // Shutdown the tailer
        bus.shutdown().await;

        // Commit a journal row AFTER shutdown. A live tailer publishes it within TAIL_INTERVAL.
        let (post_shutdown_seq, _) = commit_one(&pool).await;

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

    /// `EventBus::tailer_health()` must return the SAME `Arc` the bus handed its own tailer.
    ///
    /// This is the accessor a `/health` surface would read, and until this test existed it had
    /// zero callers and zero tests: the only health test called `tailer::spawn` directly with an
    /// `Arc` it owned. Handing the tailer a DIFFERENT `Arc` than the accessor returns therefore
    /// survived the entire 270-test suite — and an endpoint built on it would have reported zeros
    /// forever, reproducing exactly the green-while-dead mode task 016 exists to remove.
    ///
    /// Asserted as a CLIMB, not as a single non-zero reading: a counter that reaches 1 and stops is
    /// as useless for liveness as one stuck at 0, because "the tailer ran once, some time ago" and
    /// "the tailer is running" are the two states a health surface must distinguish. Both readings
    /// are taken through the accessor, so nothing but the wired-up `Arc` can satisfy them.
    #[tokio::test]
    async fn event_bus_tailer_health_tracks_the_bus_s_own_tailer() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 64).await;

        // Generous deadlines throughout: they are safety nets so a dead counter fails with a
        // diagnosis rather than hanging the suite, never the thing that decides the verdict.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);

        // First reading: the bus's tailer must have run at least one pass, observed through the
        // accessor. A bus that handed its tailer a different Arc never leaves 0 here.
        let first = loop {
            let seen = bus.tailer_health().polls_total.load(Ordering::Relaxed);
            if seen >= 1 {
                break seen;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "EventBus::tailer_health().polls_total stayed at 0 for 30s while the bus's own \
                 tailer was running; the accessor is not observing the Arc the tailer updates"
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        };

        // Second reading: it must climb STRICTLY past the first.
        loop {
            let seen = bus.tailer_health().polls_total.load(Ordering::Relaxed);
            if seen > first {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "EventBus::tailer_health().polls_total did not climb past {first} within 30s; a \
                 counter that stops advancing cannot distinguish a live tailer from a dead one"
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        bus.shutdown().await;
    }

    /// `tailer_health()`'s doc comment promises the counters are "shared across all clones of this
    /// `EventBus` exactly as `tailer_handle` is". That was a documented invariant with no test:
    /// giving the clone a fresh `TailerHealth::default()` instead of `self.tailer_health.clone()`
    /// survived the entire 275-test suite, because
    /// `event_bus_tailer_health_tracks_the_bus_s_own_tailer` never clones.
    ///
    /// It matters because `EventBus` is cloned into handlers and services — that is the whole
    /// reason `tailer_handle` is behind an `Arc` — so a `/health` surface reading a CLONE's
    /// accessor would report zeros forever while the tailer ran fine. Same green-while-dead mode as
    /// F1, one indirection further out. Task 014 creates the first real callers.
    ///
    /// The primary assertion is BEHAVIOURAL and asserted through the CLONE: a detached counter
    /// never leaves 0, and one that ticked once and stopped is as useless to a health surface as
    /// one stuck at 0, so the readings interleave — clone, then original past that, then clone past
    /// THAT. Pointer identity is asserted too, as a deterministic supplement that states the doc
    /// comment's claim literally rather than inferring it.
    #[tokio::test]
    async fn tailer_health_is_shared_with_every_clone_of_the_bus() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 64).await;
        let cloned = bus.clone();

        // The doc comment's claim, stated literally: the clone's accessor and the original's must
        // resolve to the same `TailerHealth`, which is the only way both can observe one tailer.
        assert!(
            std::ptr::eq(bus.tailer_health(), cloned.tailer_health()),
            "EventBus::clone() handed the clone a DIFFERENT TailerHealth than the original's \
             accessor returns; the counters are documented as shared across all clones exactly as \
             tailer_handle is"
        );

        // Generous deadline: a safety net so a detached counter fails with a diagnosis rather than
        // hanging the suite, never the thing that decides the verdict.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);

        // Reading 1, through the CLONE. Only the original was ever handed to the tailer, so a clone
        // holding its own `TailerHealth` never leaves 0 here.
        let via_clone = loop {
            let seen = cloned.tailer_health().polls_total.load(Ordering::Relaxed);
            if seen >= 1 {
                break seen;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the CLONE's tailer_health().polls_total stayed at 0 for 30s while the bus's tailer \
                 was running; the clone is not observing the counters the tailer updates"
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        };

        // Reading 2, through the ORIGINAL, strictly past the clone's reading.
        let via_original = loop {
            let seen = bus.tailer_health().polls_total.load(Ordering::Relaxed);
            if seen > via_clone {
                break seen;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the ORIGINAL's tailer_health().polls_total did not climb past the {via_clone} the \
                 clone observed within 30s; the two handles are not reading one climbing counter"
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        };

        // Reading 3, back through the CLONE, strictly past the original's — so the climb is
        // observed alternately through both handles rather than once through each.
        loop {
            let seen = cloned.tailer_health().polls_total.load(Ordering::Relaxed);
            if seen > via_original {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the CLONE's tailer_health().polls_total did not climb past the {via_original} the \
                 original observed within 30s; a counter that stops advancing cannot distinguish a \
                 live tailer from a dead one, whichever handle reads it"
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        // Shutdown through the clone: the tailer handle is shared too, so this stops the one tailer
        // both handles were observing.
        cloned.shutdown().await;
    }
}
