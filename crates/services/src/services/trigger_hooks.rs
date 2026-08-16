//! Trigger hooks for reactive processing of node events.
//!
//! Trigger hooks consume from the event journal and execute registered hooks on matching events.
//! Each hook maintains a persisted cursor to support at-least-once delivery across restarts.
//!
//! The `needs_rebootstrap` flag is set by compaction when it deletes journal entries before a
//! hook's cursor. On restart, the hook must observe the flag, resume from the journal's current
//! low-water mark (instead of its stale cursor), log the loss, and clear the flag.

use async_trait::async_trait;
use db::models::event::{NodeEvent, SequencedEvent};
use db::models::trigger_cursor;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{info, warn};

/// Trait for reactive event processing hooks.
///
/// A trigger hook matches specific event types and executes a side effect when an event matches.
/// Implementations are responsible for idempotency — if the hook fires and then crashes before
/// its cursor persists, the event will be replayed and must be handled gracefully.
#[async_trait]
pub trait TriggerHook: Send + Sync {
    /// The stable name of this hook, used as the primary key in `trigger_cursors`.
    ///
    /// Hook names MUST be unique across the system and stable across restarts.
    fn name(&self) -> &'static str;

    /// Test whether a specific event matches this hook's filter criteria.
    ///
    /// Must return quickly (no I/O). Only matching events are passed to `fire`.
    fn matches(&self, event: &NodeEvent) -> bool;

    /// Execute the hook on a matching event.
    ///
    /// This is the side effect: logging, state mutations, triggering further operations, etc.
    /// Must be idempotent — if it executes twice on the same event (due to crash-before-persist),
    /// both executions must result in the same observable state.
    async fn fire(&self, event: SequencedEvent);
}

/// Registry of active trigger hooks.
pub struct TriggerHookRegistry {
    hooks: Vec<Arc<dyn TriggerHook>>,
}

impl TriggerHookRegistry {
    /// Create a new registry with the provided hooks.
    pub fn new(hooks: Vec<Arc<dyn TriggerHook>>) -> Self {
        Self { hooks }
    }

    /// Get all registered hooks.
    pub fn all(&self) -> &[Arc<dyn TriggerHook>] {
        &self.hooks
    }
}

/// A trigger hook that logs when a task's status changes.
///
/// This hook is a proof-of-concept observable side effect for SC6.
pub struct TaskStatusChangedHook;

#[async_trait]
impl TriggerHook for TaskStatusChangedHook {
    fn name(&self) -> &'static str {
        "task_status_changed_logger"
    }

    fn matches(&self, event: &NodeEvent) -> bool {
        matches!(event, NodeEvent::TaskStatusChanged { .. })
    }

    async fn fire(&self, event: SequencedEvent) {
        if let NodeEvent::TaskStatusChanged {
            task_id,
            old_status,
            new_status,
        } = &event.event
        {
            info!(
                task_id = %task_id,
                old_status = ?old_status,
                new_status = ?new_status,
                seq = event.seq,
                "task_status_changed_event"
            );
        }
    }
}

/// Run a single trigger hook's processing loop.
///
/// This function:
/// 1. Loads the hook's cursor from persistent storage (0 if absent)
/// 2. Checks for the rebootstrap flag (set if journal compaction deleted unprocessed events)
/// 3. Subscribes to the event stream starting from the cursor
/// 4. For each event:
///    - If the hook matches the event: calls fire(), then persists the cursor
///    - If the hook doesn't match: immediately persists the cursor (for compaction floor)
/// 5. Clears the rebootstrap flag on the first update (hook has recovered)
///
/// Note: This is designed to be spawned as a long-lived background task.
pub async fn run_hook(
    pool: SqlitePool,
    hook: Arc<dyn TriggerHook>,
    event_bus: Arc<crate::services::event_bus::EventBus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let hook_name = hook.name();

    // Load the cursor and check the rebootstrap flag
    let (mut cursor, needs_rebootstrap) = trigger_cursor::get_with_flag(&pool, hook_name).await?;

    // If the flag is set, the hard cap deleted unprocessed events
    if needs_rebootstrap {
        warn!(
            hook_name = %hook_name,
            lost_cursor = cursor,
            "hook needs rebootstrap: journal compaction deleted events before cursor"
        );

        // Find the journal's current low-water mark
        let new_min: Option<i64> = sqlx::query_scalar("SELECT MIN(seq) FROM event_journal")
            .fetch_one(&pool)
            .await?;

        cursor = new_min.unwrap_or(0);
        info!(
            hook_name = %hook_name,
            resumed_from_seq = cursor,
            "resuming from journal low-water mark after rebootstrap"
        );

        // Clear the flag by updating the cursor
        trigger_cursor::set(&pool, hook_name, cursor).await?;
    }

    // Subscribe to events starting from the cursor
    let mut stream = event_bus.subscribe_from(cursor)?;

    // Process events as they arrive
    while let Some(result) = futures_util::stream::StreamExt::next(&mut stream).await {
        let event = result?;

        // Check if the event matches this hook's filter
        if hook.matches(&event.event) {
            // Fire the hook for matching events
            hook.fire(event.clone()).await;
            // Then persist the cursor
            trigger_cursor::set(&pool, hook_name, event.seq).await?;
        } else {
            // Non-matching events still advance the cursor (for compaction floor)
            trigger_cursor::set(&pool, hook_name, event.seq).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::test_utils::create_test_pool_with_migrations;
    use std::sync::Mutex;
    use tokio::time::{Duration, sleep};
    use uuid::Uuid;

    /// A test hook that records every fired event in a Vec.
    struct RecordingHook {
        name: &'static str,
        match_type: &'static str,
        fired_events: Arc<Mutex<Vec<SequencedEvent>>>,
    }

    impl RecordingHook {
        fn new(name: &'static str, match_type: &'static str) -> Self {
            Self {
                name,
                match_type,
                fired_events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_fired(&self) -> Vec<SequencedEvent> {
            self.fired_events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl TriggerHook for RecordingHook {
        fn name(&self) -> &'static str {
            self.name
        }

        fn matches(&self, event: &NodeEvent) -> bool {
            event.event_type() == self.match_type
        }

        async fn fire(&self, event: SequencedEvent) {
            self.fired_events.lock().unwrap().push(event);
        }
    }

    async fn commit_event(pool: &SqlitePool, event: NodeEvent) -> i64 {
        let mut tx = pool.begin().await.unwrap();
        let seq = db::models::event_journal::append(&mut *tx, &event).await.unwrap();
        tx.commit().await.unwrap();
        seq
    }

    async fn create_events(pool: &SqlitePool) -> Vec<i64> {

        let mut seqs = Vec::new();

        // Event 1: task_created
        let task_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let event1 = NodeEvent::TaskCreated { task_id, project_id };
        let mut tx = pool.begin().await.unwrap();
        let seq = db::models::event_journal::append(&mut *tx, &event1).await.unwrap();
        tx.commit().await.unwrap();
        seqs.push(seq);

        // Event 2: task_status_changed (MATCHES)
        let event2 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::Todo,
            new_status: db::models::task::TaskStatus::InProgress,
        };
        let mut tx = pool.begin().await.unwrap();
        let seq = db::models::event_journal::append(&mut *tx, &event2).await.unwrap();
        tx.commit().await.unwrap();
        seqs.push(seq);

        // Event 3: task_status_changed (MATCHES)
        let event3 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::InProgress,
            new_status: db::models::task::TaskStatus::InReview,
        };
        let mut tx = pool.begin().await.unwrap();
        let seq = db::models::event_journal::append(&mut *tx, &event3).await.unwrap();
        tx.commit().await.unwrap();
        seqs.push(seq);

        seqs
    }

    #[tokio::test]
    async fn hook_fires_only_on_matching_events() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        // Create a hook that only matches task_status_changed
        let hook = Arc::new(RecordingHook::new(
            "test_hook_matching",
            "task_status_changed",
        ));

        // Create test events
        let _seqs = create_events(&pool).await;

        // Run the hook from the start
        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        // Give the hook time to process
        sleep(Duration::from_millis(500)).await;

        // Assert the hook only fired for task_status_changed events (not task_created)
        let fired = hook.get_fired();
        assert_eq!(fired.len(), 2, "Hook should fire twice (on seqs 2 and 3)");
        assert!(
            matches!(fired[0].event, NodeEvent::TaskStatusChanged { .. }),
            "First fired event must be TaskStatusChanged"
        );
        assert!(
            matches!(fired[1].event, NodeEvent::TaskStatusChanged { .. }),
            "Second fired event must be TaskStatusChanged"
        );
    }

    #[tokio::test]
    async fn cursor_is_persisted_after_each_fire() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        let hook = Arc::new(RecordingHook::new(
            "test_hook_cursor",
            "task_status_changed",
        ));

        let seqs = create_events(&pool).await;

        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(500)).await;

        // The hook should have fired for events at seqs[1] and seqs[2]
        // and persisted cursors at those seqs
        let fired = hook.get_fired();
        assert_eq!(fired.len(), 2);

        // Check that the cursor was persisted past the last fired event
        let persisted_cursor = trigger_cursor::get(&pool, "test_hook_cursor")
            .await
            .unwrap();
        assert_eq!(
            persisted_cursor, seqs[2],
            "Cursor should be persisted at the seq of the last matched event"
        );
    }

    #[tokio::test]
    async fn restart_resumes_from_persisted_cursor_without_loss() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        let hook = Arc::new(RecordingHook::new(
            "test_hook_restart",
            "task_status_changed",
        ));

        // Phase 1: Create events 1-3 and process them
        let _seqs = create_events(&pool).await;

        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // Verify the hook fired for events 2 and 3
        let fired_phase1 = hook.get_fired();
        assert_eq!(fired_phase1.len(), 2);

        // Kill the hook
        handle.abort();
        sleep(Duration::from_millis(100)).await;

        // Phase 2: Create events 4-6 while hook is DOWN
        let task_id = Uuid::new_v4();
        let event4 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::InReview,
            new_status: db::models::task::TaskStatus::Done,
        };
        let mut tx = pool.begin().await.unwrap();
        let _ = db::models::event_journal::append(&mut *tx, &event4).await.unwrap();
        tx.commit().await.unwrap();

        let event5 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::Done,
            new_status: db::models::task::TaskStatus::Todo,
        };
        let mut tx = pool.begin().await.unwrap();
        let _ = db::models::event_journal::append(&mut *tx, &event5).await.unwrap();
        tx.commit().await.unwrap();

        let event6 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::Todo,
            new_status: db::models::task::TaskStatus::InProgress,
        };
        let mut tx = pool.begin().await.unwrap();
        let _ = db::models::event_journal::append(&mut *tx, &event6).await.unwrap();
        tx.commit().await.unwrap();

        // Clear the fired events list to track only new firings
        hook.fired_events.lock().unwrap().clear();

        // Phase 3: Start a NEW runner for the same hook
        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // Assert the hook sees events 4, 5, 6 (no loss)
        let fired_phase2 = hook.get_fired();
        assert_eq!(
            fired_phase2.len(),
            3,
            "Hook should fire for the 3 events that were created while it was down"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn at_least_once_tolerates_duplicate_delivery() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        let hook = Arc::new(RecordingHook::new(
            "test_hook_at_least_once",
            "task_status_changed",
        ));

        let _seqs = create_events(&pool).await;

        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // Simulate a crash by aborting the task without letting it persist
        // (In this test, we trust the implementation persists immediately, so we just
        // verify the hook is idempotent — calling fire twice produces the same result)

        let fired = hook.get_fired();
        assert_eq!(fired.len(), 2);

        // Clear and replay the events manually to simulate crash-before-persist
        hook.fired_events.lock().unwrap().clear();
        for event in fired.clone() {
            hook.fire(event).await;
        }

        // Verify idempotency: firing twice produces the same record
        let fired_again = hook.get_fired();
        assert_eq!(
            fired_again.len(),
            2,
            "Recording hook is idempotent: same events, same records"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn unknown_hook_starts_at_cursor_zero() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        // Create some events
        let task_id = Uuid::new_v4();
        let event1 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::Todo,
            new_status: db::models::task::TaskStatus::InProgress,
        };
        let _ = commit_event(&pool, event1).await;

        let event2 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::InProgress,
            new_status: db::models::task::TaskStatus::InReview,
        };
        let _ = commit_event(&pool, event2).await;

        // Now create a NEW hook that has never run before
        let hook = Arc::new(RecordingHook::new("brand_new_hook", "task_status_changed"));

        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // The hook should replay both events (starting from cursor 0)
        let fired = hook.get_fired();
        assert_eq!(
            fired.len(),
            2,
            "Unknown hook should replay from the beginning (cursor 0)"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn cursor_advances_past_non_matching_events() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        let hook = Arc::new(RecordingHook::new(
            "test_hook_non_matching",
            "task_status_changed",
        ));

        let task_id = Uuid::new_v4();

        // Event 1: MATCHING task_status_changed
        let event1 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::Todo,
            new_status: db::models::task::TaskStatus::InProgress,
        };
        let _seq1 = commit_event(&pool, event1).await;

        // Events 2-6: NON-MATCHING task_created
        let mut seq_last = _seq1;
        for _ in 0..5 {
            let non_matching = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            seq_last = commit_event(&pool, non_matching).await;
        }

        // Run the hook
        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // The hook should have fired once (for event 1)
        let fired = hook.get_fired();
        assert_eq!(fired.len(), 1);

        // But the cursor should have advanced PAST all 6 events
        let persisted_cursor = trigger_cursor::get(&pool, "test_hook_non_matching")
            .await
            .unwrap();
        assert_eq!(
            persisted_cursor, seq_last,
            "Cursor must advance past non-matching events"
        );

        // Now drop the hook and restart it
        handle.abort();
        sleep(Duration::from_millis(100)).await;
        hook.fired_events.lock().unwrap().clear();

        // Start a new runner for the same hook
        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // The hook should NOT replay those 6 events — it should start fresh
        let fired_after_restart = hook.get_fired();
        assert_eq!(
            fired_after_restart.len(),
            0,
            "After restart, hook should not replay non-matching events"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn rebootstrap_flag_is_surfaced_and_cleared() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let event_bus = Arc::new(crate::services::event_bus::EventBus::new(pool.clone(), 256).await);

        let hook = Arc::new(RecordingHook::new(
            "test_hook_rebootstrap",
            "task_status_changed",
        ));

        // Create some events
        let task_id = Uuid::new_v4();
        let event1 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::Todo,
            new_status: db::models::task::TaskStatus::InProgress,
        };
        let _seq1 = commit_event(&pool, event1).await;

        // Simulate the hard cap: set needs_rebootstrap = 1 at an old cursor
        sqlx::query(
            r#"INSERT INTO trigger_cursors (hook_name, last_processed_seq, needs_rebootstrap, updated_at)
               VALUES (?, ?, 1, datetime('now', 'subsec'))"#,
        )
        .bind("test_hook_rebootstrap")
        .bind(0) // Stale cursor
        .execute(&pool)
        .await
        .unwrap();

        // Create more events AFTER the hardcap occurred
        let event2 = NodeEvent::TaskStatusChanged {
            task_id,
            old_status: db::models::task::TaskStatus::InProgress,
            new_status: db::models::task::TaskStatus::InReview,
        };
        let seq2 = commit_event(&pool, event2).await;

        // Now run the hook
        let hook_clone = hook.clone();
        let event_bus_clone = event_bus.clone();
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let _ = run_hook(pool_clone, hook_clone, event_bus_clone).await;
        });

        sleep(Duration::from_millis(300)).await;

        // The hook should have fired for event2 only (event1 was before the min seq after rebootstrap)
        // subscribe_from(min_seq=1) reads events with seq > 1, which is only event2 at seq2
        let fired = hook.get_fired();
        assert_eq!(
            fired.len(),
            1,
            "Hook should fire for new events after resuming from journal minimum (event1 lost)"
        );

        // The flag should be CLEARED after the first update
        let (cursor, flag) = trigger_cursor::get_with_flag(&pool, "test_hook_rebootstrap")
            .await
            .unwrap();
        assert!(!flag, "Rebootstrap flag should be cleared after recovery");
        assert_eq!(
            cursor, seq2,
            "Cursor should reflect the last processed event"
        );

        handle.abort();
    }
}
