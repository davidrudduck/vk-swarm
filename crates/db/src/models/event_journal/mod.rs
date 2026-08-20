//! Event journal model for storing durable, ordered events.
//!
//! The event journal provides at-least-once delivery guarantees for lifecycle events.
//! Each event is assigned a monotonically increasing sequence number used by consumers
//! to resume from a known point.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

mod queries;
pub use queries::{append, compact, high_water_mark, low_water_mark, read_range};

/// Error type for event journal operations.
#[derive(Debug, thiserror::Error)]
pub enum EventJournalError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("event payload serialization failed: {0}")]
    Serde(#[from] serde_json::Error),
}

/// A single entry in the event journal.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EventJournalEntry {
    pub seq: i64,
    pub event_type: String,
    pub payload: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::NodeEvent;
    use crate::test_utils::create_test_pool_with_migrations;
    use uuid::Uuid;

    #[tokio::test]
    async fn append_in_transaction_assigns_monotonic_seq() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let mut tx = pool.begin().await.unwrap();

        let event1 = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let event2 = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let event3 = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };

        let seq1 = append(&mut *tx, &event1).await.unwrap();
        let seq2 = append(&mut *tx, &event2).await.unwrap();
        let seq3 = append(&mut *tx, &event3).await.unwrap();

        tx.commit().await.unwrap();

        assert!(seq1 < seq2, "seq1 should be less than seq2");
        assert!(seq2 < seq3, "seq2 should be less than seq3");
    }

    #[tokio::test]
    async fn rollback_journals_nothing() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = append(&mut *tx, &event).await.unwrap();
            // Intentionally drop tx without committing
            drop(tx);
        }

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(count.0, 0, "rollback should journal nothing");
    }

    #[tokio::test]
    async fn committed_seqs_are_strictly_increasing_across_rollback() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // First append and commit
        let mut tx1 = pool.begin().await.unwrap();
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let seq1 = append(&mut *tx1, &event).await.unwrap();
        tx1.commit().await.unwrap();

        // Second append and rollback
        let mut tx2 = pool.begin().await.unwrap();
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let _seq_rolled_back = append(&mut *tx2, &event).await.unwrap();
        drop(tx2); // Rollback without commit

        // Third append and commit
        let mut tx3 = pool.begin().await.unwrap();
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let seq3 = append(&mut *tx3, &event).await.unwrap();
        tx3.commit().await.unwrap();

        // Both committed seqs must be strictly increasing
        assert!(seq1 < seq3, "committed seqs must be strictly increasing");

        // Verify only two rows in journal (seq1 and seq3)
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 2, "only committed events should be in journal");
    }

    #[tokio::test]
    async fn range_read_returns_exclusive_lower_inclusive_upper() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Append 5 events
        let mut tx = pool.begin().await.unwrap();
        for _ in 0..5 {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = append(&mut *tx, &event).await.unwrap();
        }
        tx.commit().await.unwrap();

        // Read (2, 4] should return exactly seqs 3 and 4
        let events = read_range(&pool, 2, 4).await.unwrap();

        assert_eq!(
            events.len(),
            2,
            "should have exactly 2 events in range (2, 4]"
        );
        assert_eq!(events[0].seq, 3, "first event should have seq 3");
        assert_eq!(events[1].seq, 4, "second event should have seq 4");
        assert!(
            events[0].seq < events[1].seq,
            "events should be seq-ordered"
        );
    }

    #[tokio::test]
    async fn range_read_is_empty_above_high_water() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Append 5 events
        let mut tx = pool.begin().await.unwrap();
        for _ in 0..5 {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = append(&mut *tx, &event).await.unwrap();
        }
        tx.commit().await.unwrap();

        // Read (5, 5] should be empty
        let events = read_range(&pool, 5, 5).await.unwrap();

        assert_eq!(events.len(), 0, "range above high water should be empty");
    }

    #[tokio::test]
    async fn compact_respects_retention_floor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Insert rows with backdated created_at
        let old_time = Utc::now() - chrono::Duration::hours(48);
        let recent_time = Utc::now();

        sqlx::query(
            "INSERT INTO event_journal (event_type, payload, created_at) VALUES (?, ?, ?)",
        )
        .bind("task_created")
        .bind(r#"{"type":"task_created","task_id":"00000000-0000-0000-0000-000000000001","project_id":"00000000-0000-0000-0000-000000000002"}"#)
        .bind(old_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO event_journal (event_type, payload, created_at) VALUES (?, ?, ?)",
        )
        .bind("task_created")
        .bind(r#"{"type":"task_created","task_id":"00000000-0000-0000-0000-000000000003","project_id":"00000000-0000-0000-0000-000000000004"}"#)
        .bind(recent_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
        .execute(&pool)
        .await
        .unwrap();

        // Compact with 24-hour retention
        let retention_hours = 24;
        let min_rows = 1;
        let max_rows = 100;
        compact(&pool, retention_hours, min_rows, max_rows)
            .await
            .unwrap();

        // Old row should be gone, recent should remain
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1, "old row should be deleted by compaction");
    }

    /// Regression: the retention cutoff must be bound in SQLite's stored text format.
    /// created_at defaults to datetime('now','subsec') ("YYYY-MM-DD HH:MM:SS.SSS"); an RFC 3339
    /// cutoff ("YYYY-MM-DDTHH:MM:SS+00:00") collates ABOVE every same-day stored value ('T' > ' '),
    /// which deleted rows that were still inside the retention window.
    #[tokio::test]
    async fn compact_keeps_same_day_rows_inside_the_retention_window() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Two rows written the production way (created_at = datetime('now','subsec')), then one
        // backdated 30 minutes — still inside a 1-hour retention window, on the same calendar day.
        let mut tx = pool.begin().await.unwrap();
        for _ in 0..2 {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = append(&mut *tx, &event).await.unwrap();
        }
        tx.commit().await.unwrap();

        sqlx::query(
            "UPDATE event_journal SET created_at = datetime('now', '-30 minutes', 'subsec')              WHERE seq = (SELECT MIN(seq) FROM event_journal)",
        )
        .execute(&pool)
        .await
        .unwrap();

        // retention 1h, min_rows 1: the 30-minute-old row is deletable by every guard EXCEPT the
        // retention window, so it survives only if the cutoff comparison is format-correct.
        compact(&pool, 1, 1, 100).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count.0, 2,
            "a same-day row inside the retention window must survive compaction"
        );
    }

    #[tokio::test]
    async fn compact_never_crosses_min_trigger_cursor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Insert trigger cursor with floor at 3
        sqlx::query("INSERT INTO trigger_cursors (hook_name, last_processed_seq) VALUES (?, ?)")
            .bind("test_hook")
            .bind(3)
            .execute(&pool)
            .await
            .unwrap();

        // Insert journal rows with backdated created_at beyond retention
        let old_time = Utc::now() - chrono::Duration::hours(48);
        for i in 0..5 {
            sqlx::query(
                "INSERT INTO event_journal (event_type, payload, created_at) VALUES (?, ?, ?)",
            )
            .bind("task_created")
            .bind(format!(
                r#"{{"type":"task_created","task_id":"00000000-0000-0000-0000-00000000000{}","project_id":"00000000-0000-0000-0000-00000000000{}"}}"#,
                i, i
            ))
            .bind(old_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
            .execute(&pool)
            .await
            .unwrap();
        }

        // Compact with 24-hour retention, min_rows=1 so cursor floor is the only real protection
        compact(&pool, 24, 1, 100).await.unwrap();

        // Rows with seq >= 3 (cursor floor) should survive; assert exact set, not just a predicate
        let surviving_seqs: Vec<i64> =
            sqlx::query_scalar("SELECT seq FROM event_journal ORDER BY seq")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(
            surviving_seqs,
            vec![3, 4, 5],
            "cursor floor must protect rows 3,4,5 exactly"
        );
    }

    #[tokio::test]
    async fn compact_retains_min_rows_floor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Insert 10 rows with backdated created_at (all beyond retention)
        let old_time = Utc::now() - chrono::Duration::hours(48);
        for i in 0..10 {
            sqlx::query(
                "INSERT INTO event_journal (event_type, payload, created_at) VALUES (?, ?, ?)",
            )
            .bind("task_created")
            .bind(format!(
                r#"{{"type":"task_created","task_id":"00000000-0000-0000-0000-00000000000{}","project_id":"00000000-0000-0000-0000-00000000000{}"}}"#,
                i, i
            ))
            .bind(old_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
            .execute(&pool)
            .await
            .unwrap();
        }

        // Compact with retention expired for everything, but min_rows = 5
        compact(&pool, 24, 5, 100).await.unwrap();

        // At least 5 rows should survive (min_rows floor)
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(count.0 >= 5, "min_rows floor should be respected");
    }

    #[tokio::test]
    async fn append_composes_with_a_caller_owned_transaction() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Test passes a caller-owned transaction to append
        let mut tx = pool.begin().await.unwrap();

        let event1 = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let event2 = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };

        let seq1 = append(&mut *tx, &event1).await.unwrap();
        let seq2 = append(&mut *tx, &event2).await.unwrap();

        // Commit in the test
        tx.commit().await.unwrap();

        // Verify both rows are present
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 2, "both appends should be committed");
        assert!(seq1 < seq2, "seqs should be monotonic");
    }

    #[tokio::test]
    async fn compact_treats_a_zero_cursor_as_a_real_floor() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Insert exactly one trigger cursor with last_processed_seq = 0 (freshly-registered hook)
        sqlx::query("INSERT INTO trigger_cursors (hook_name, last_processed_seq) VALUES (?, ?)")
            .bind("new_hook")
            .bind(0)
            .execute(&pool)
            .await
            .unwrap();

        // Insert journal rows with backdated created_at beyond retention
        let old_time = Utc::now() - chrono::Duration::hours(48);
        for i in 0..5 {
            sqlx::query(
                "INSERT INTO event_journal (event_type, payload, created_at) VALUES (?, ?, ?)",
            )
            .bind("task_created")
            .bind(format!(
                r#"{{"type":"task_created","task_id":"00000000-0000-0000-0000-00000000000{}","project_id":"00000000-0000-0000-0000-00000000000{}"}}"#,
                i, i
            ))
            .bind(old_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
            .execute(&pool)
            .await
            .unwrap();
        }

        // Compact with small min_rows so cursor floor is the only real protection
        compact(&pool, 24, 1, 100).await.unwrap();

        // With cursor at 0, seq < 0 is never true, so ALL rows should survive
        let surviving_seqs: Vec<i64> =
            sqlx::query_scalar("SELECT seq FROM event_journal ORDER BY seq")
                .fetch_all(&pool)
                .await
                .unwrap();

        // The 5 rows inserted will have seqs 1-5 (autoincrement from 1)
        assert_eq!(
            surviving_seqs,
            vec![1, 2, 3, 4, 5],
            "all rows should survive with cursor at 0"
        );
    }

    #[tokio::test]
    async fn hard_cap_overrides_cursor_floor_and_flags_rebootstrap() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Insert a trigger cursor with low last_processed_seq (will be below new minimum)
        sqlx::query("INSERT INTO trigger_cursors (hook_name, last_processed_seq) VALUES (?, ?)")
            .bind("test_hook_below")
            .bind(5)
            .execute(&pool)
            .await
            .unwrap();

        // Insert a trigger cursor with high last_processed_seq (will be above new minimum)
        sqlx::query("INSERT INTO trigger_cursors (hook_name, last_processed_seq) VALUES (?, ?)")
            .bind("test_hook_above")
            .bind(20)
            .execute(&pool)
            .await
            .unwrap();

        // Insert more than max_rows journal rows above the cursor floor
        let max_rows = 10;
        for i in 0..(max_rows + 15) {
            sqlx::query(
                "INSERT INTO event_journal (event_type, payload) VALUES (?, ?)",
            )
            .bind("task_created")
            .bind(format!(
                r#"{{"type":"task_created","task_id":"00000000-0000-0000-0000-00000000000{}","project_id":"00000000-0000-0000-0000-00000000000{}"}}"#,
                i, i
            ))
            .execute(&pool)
            .await
            .unwrap();
        }

        // Compact with hard cap
        // Rows 1-15 will be deleted (hard cap), new minimum surviving seq = 16
        compact(&pool, 24, 1, max_rows as i64).await.unwrap();

        // (a) Row count should drop to at most max_rows
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM event_journal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            count.0 <= max_rows as i64,
            "hard cap should limit to max_rows"
        );

        // (b) Rows below cursor floor should be deleted (assert a specific seq below floor is absent)
        let row_below_floor: Option<i64> =
            sqlx::query_scalar("SELECT seq FROM event_journal WHERE seq = ?")
                .bind(4)
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(
            row_below_floor.is_none(),
            "row 4 (below cursor floor 5) should be absent after hard cap forces deletion"
        );

        // (c) Cursor below new minimum (16) should be flagged
        let needs_rebootstrap_below: (i64,) =
            sqlx::query_as("SELECT needs_rebootstrap FROM trigger_cursors WHERE hook_name = ?")
                .bind("test_hook_below")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            needs_rebootstrap_below.0, 1,
            "cursor below new minimum should be flagged"
        );

        // (d) Cursor above new minimum (16) should NOT be flagged
        let needs_rebootstrap_above: (i64,) =
            sqlx::query_as("SELECT needs_rebootstrap FROM trigger_cursors WHERE hook_name = ?")
                .bind("test_hook_above")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            needs_rebootstrap_above.0, 0,
            "cursor above new minimum should NOT be flagged"
        );
    }

    /// Regression: a cursor at new_min_seq - 1 lost nothing — it resumes at seq > cursor,
    /// which is exactly the first retained row — so it must NOT be flagged for rebootstrap.
    /// Only cursors below new_min_seq - 1 have a deleted, unprocessed event between the
    /// cursor and the low-water mark.
    #[tokio::test]
    async fn hard_cap_does_not_rebootstrap_a_cursor_at_the_retained_boundary() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Boundary cursor: after hard-cap deletion the new minimum surviving seq is 16,
        // so a cursor at 15 resumes gaplessly at 16.
        sqlx::query("INSERT INTO trigger_cursors (hook_name, last_processed_seq) VALUES (?, ?)")
            .bind("test_hook_boundary")
            .bind(15)
            .execute(&pool)
            .await
            .unwrap();

        // One below the boundary: seq 15 existed, was deleted, and was never processed.
        sqlx::query("INSERT INTO trigger_cursors (hook_name, last_processed_seq) VALUES (?, ?)")
            .bind("test_hook_lost")
            .bind(14)
            .execute(&pool)
            .await
            .unwrap();

        let max_rows = 10;
        for i in 0..(max_rows + 15) {
            sqlx::query("INSERT INTO event_journal (event_type, payload) VALUES (?, ?)")
                .bind("task_created")
                .bind(format!(
                    r#"{{"type":"task_created","task_id":"00000000-0000-0000-0000-00000000000{}","project_id":"00000000-0000-0000-0000-00000000000{}"}}"#,
                    i, i
                ))
                .execute(&pool)
                .await
                .unwrap();
        }

        // Hard cap deletes seqs 1-15; new minimum surviving seq = 16.
        compact(&pool, 24, 1, max_rows as i64).await.unwrap();

        let boundary: (i64,) =
            sqlx::query_as("SELECT needs_rebootstrap FROM trigger_cursors WHERE hook_name = ?")
                .bind("test_hook_boundary")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            boundary.0, 0,
            "a cursor at new_min_seq - 1 resumes gaplessly and must not be flagged"
        );

        let lost: (i64,) =
            sqlx::query_as("SELECT needs_rebootstrap FROM trigger_cursors WHERE hook_name = ?")
                .bind("test_hook_lost")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            lost.0, 1,
            "a cursor with a deleted unprocessed event between it and the low-water mark must be flagged"
        );
    }
}
