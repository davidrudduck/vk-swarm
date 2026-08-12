//! Database queries for the event journal model.

use crate::models::event::{NodeEvent, SequencedEvent};
use serde_json;
use sqlx::{Executor, SqlitePool};

use super::EventJournalError;

/// Append an event to the journal, returning its assigned sequence number.
///
/// This operation is generic over the executor type to support both opening a new
/// transaction and composing with a caller-owned transaction. This is load-bearing
/// for `Task::delete` which already owns an outer transaction spanning child
/// nullification.
///
/// The operation does NOT commit — committing is the caller's responsibility in
/// both composition modes.
pub async fn append<'e, E>(executor: E, event: &NodeEvent) -> Result<i64, EventJournalError>
where
    E: Executor<'e, Database = sqlx::Sqlite>,
{
    let payload = serde_json::to_string(event)?;
    let event_type = event.event_type();

    let seq = sqlx::query_scalar::<_, i64>(
        "INSERT INTO event_journal (event_type, payload) VALUES (?, ?) RETURNING seq",
    )
    .bind(event_type)
    .bind(payload)
    .fetch_one(executor)
    .await?;

    Ok(seq)
}

/// Read events from the journal in a range (exclusive lower, inclusive upper).
///
/// This returns the `(cursor, mark]` window used by the spec's replay algorithm:
/// `after_seq < seq <= through_seq`.
pub async fn read_range(
    pool: &SqlitePool,
    after_seq: i64,
    through_seq: i64,
) -> Result<Vec<SequencedEvent>, EventJournalError> {
    let rows = sqlx::query_as::<_, super::EventJournalEntry>(
        "SELECT seq, event_type, payload, created_at FROM event_journal WHERE seq > ? AND seq <= ? ORDER BY seq ASC",
    )
    .bind(after_seq)
    .bind(through_seq)
    .fetch_all(pool)
    .await?;

    let mut events = Vec::new();
    for row in rows {
        let event: NodeEvent = serde_json::from_str(&row.payload)?;
        events.push(SequencedEvent { seq: row.seq, event });
    }

    Ok(events)
}

/// Get the highest sequence number in the journal (high water mark).
pub async fn high_water_mark(pool: &SqlitePool) -> Result<i64, EventJournalError> {
    let mark = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(seq), 0) FROM event_journal",
    )
    .fetch_one(pool)
    .await?;

    Ok(mark)
}

/// Compact the event journal, removing old entries while respecting constraints.
///
/// This is a two-stage process:
/// 1. **Normal pass**: Delete rows that are BOTH:
///    - Older than `retention_hours`
///    - Outside the newest `min_rows`
///    - Strictly below the cursor floor (MIN(last_processed_seq) from trigger_cursors,
///      or the high water mark if trigger_cursors is empty)
///
/// 2. **Hard cap**: If the journal still holds more than `max_rows` rows:
///    - Delete the oldest rows down to `max_rows` (ignoring cursor floor)
///    - Set `needs_rebootstrap = 1` on every trigger_cursor whose `last_processed_seq`
///      is below the new minimum surviving `seq`
///
/// Returns the number of rows deleted.
pub async fn compact(
    pool: &SqlitePool,
    retention_hours: i64,
    min_rows: i64,
    max_rows: i64,
) -> Result<u64, EventJournalError> {
    let mut tx = pool.begin().await?;

    // Get high water mark for cursor floor calculation
    let high_water = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(seq), 0) FROM event_journal",
    )
    .fetch_one(&mut *tx)
    .await?;

    // Determine the cursor floor: MIN(last_processed_seq) from trigger_cursors,
    // or high_water if no cursors exist. Decide on Option, not the value 0.
    let cursor_floor_option: Option<i64> = sqlx::query_scalar(
        "SELECT MIN(last_processed_seq) FROM trigger_cursors",
    )
    .fetch_one(&mut *tx)
    .await?;

    let cursor_floor = cursor_floor_option.unwrap_or(high_water);

    // Stage 1: Normal pass (respect cursor floor and retention window)
    let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(retention_hours);

    // Delete rows that are old, below min_rows, and strictly below cursor floor
    let stage1_result = sqlx::query(
        r#"
        DELETE FROM event_journal
        WHERE seq < ?
          AND seq < (
            -- Find the seq at the boundary of min_rows
            SELECT seq FROM (
              SELECT seq FROM event_journal
              ORDER BY seq DESC
              LIMIT ?
            ) AS newest_rows
            ORDER BY seq ASC
            LIMIT 1
          )
          AND created_at < ?
        "#,
    )
    .bind(cursor_floor)
    .bind(min_rows)
    .bind(cutoff_time.to_rfc3339())
    .execute(&mut *tx)
    .await?;

    let stage1_deleted = stage1_result.rows_affected();

    // Stage 2: Hard cap (if still too many rows, delete oldest regardless of cursor)
    let current_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM event_journal",
    )
    .fetch_one(&mut *tx)
    .await?;

    if current_count > max_rows {
        // Delete oldest rows down to max_rows
        let rows_to_delete = current_count - max_rows;
        let stage2_result = sqlx::query(
            r#"
            DELETE FROM event_journal
            WHERE seq IN (
              SELECT seq FROM event_journal
              ORDER BY seq ASC
              LIMIT ?
            )
            "#,
        )
        .bind(rows_to_delete)
        .execute(&mut *tx)
        .await?;

        let stage2_deleted = stage2_result.rows_affected();

        // Get the new minimum seq
        let new_min_seq = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT MIN(seq) FROM event_journal",
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);

        // Flag cursors that are below the new minimum
        sqlx::query(
            "UPDATE trigger_cursors SET needs_rebootstrap = 1 WHERE last_processed_seq < ?",
        )
        .bind(new_min_seq)
        .execute(&mut *tx)
        .await?;

        // Total deleted = stage 1 + stage 2
        let total_deleted = stage1_deleted + stage2_deleted;
        tx.commit().await?;
        Ok(total_deleted)
    } else {
        tx.commit().await?;
        Ok(stage1_deleted)
    }
}
