//! Persisted cursor tracking for trigger hooks.
//!
//! Each hook maintains its own cursor in the `trigger_cursors` table to enable at-least-once
//! delivery across restarts. When a hook processes an event, it advances its cursor past that
//! event's sequence number — for matching events after firing, for non-matching events
//! immediately. This ensures the hook does not replay already-seen events while the journal
//! remains unpacked.
//!
//! The `needs_rebootstrap` flag is set by compaction when it deletes journal rows before a
//! hook's cursor. On restart, the hook must observe the flag, resume from the journal's current
//! low-water mark (instead of its stale cursor), log the loss, and clear the flag.

use sqlx::SqlitePool;

/// Get the last processed sequence number for a hook.
///
/// Returns the hook's cursor if it exists, or 0 if this is the first time the hook has run.
pub async fn get(pool: &SqlitePool, hook_name: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"SELECT COALESCE(last_processed_seq, 0) FROM trigger_cursors WHERE hook_name = ?"#,
    )
    .bind(hook_name)
    .fetch_optional(pool)
    .await?;

    Ok(row.unwrap_or(0))
}

/// Get the last processed sequence number and the rebootstrap flag for a hook.
///
/// Returns (cursor, needs_rebootstrap). If the hook doesn't exist, returns (0, false).
pub async fn get_with_flag(pool: &SqlitePool, hook_name: &str) -> Result<(i64, bool), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct CursorRow {
        last_processed_seq: i64,
        needs_rebootstrap: i64,
    }

    let row = sqlx::query_as::<_, CursorRow>(
        r#"SELECT last_processed_seq, needs_rebootstrap FROM trigger_cursors WHERE hook_name = ?"#,
    )
    .bind(hook_name)
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|r| (r.last_processed_seq, r.needs_rebootstrap != 0))
        .unwrap_or((0, false)))
}

/// Update (or insert) the cursor for a hook.
///
/// This performs an UPSERT: if a row exists for `hook_name`, it updates `last_processed_seq`
/// and `updated_at`. If no row exists, it inserts a new one. The `needs_rebootstrap` flag
/// is cleared on every update (the hook has resumed from the correct position).
pub async fn set(pool: &SqlitePool, hook_name: &str, seq: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO trigger_cursors (hook_name, last_processed_seq, needs_rebootstrap, updated_at)
           VALUES (?, ?, 0, datetime('now', 'subsec'))
           ON CONFLICT(hook_name) DO UPDATE SET last_processed_seq = excluded.last_processed_seq,
                                                  needs_rebootstrap = 0,
                                                  updated_at = datetime('now', 'subsec')"#,
    )
    .bind(hook_name)
    .bind(seq)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get the minimum cursor across all hooks.
///
/// Used by compaction to determine which journal rows can be deleted. Returns None if the
/// table is empty.
pub async fn min_cursor(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let min = sqlx::query_scalar::<_, Option<i64>>(
        r#"SELECT MIN(last_processed_seq) FROM trigger_cursors"#,
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cursor_get_missing_returns_zero() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        let cursor = get(&pool, "test_hook").await.unwrap();
        assert_eq!(cursor, 0);
    }

    #[tokio::test]
    async fn test_cursor_set_and_get() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        set(&pool, "test_hook", 42).await.unwrap();
        let cursor = get(&pool, "test_hook").await.unwrap();
        assert_eq!(cursor, 42);
    }

    #[tokio::test]
    async fn test_cursor_set_updates_existing() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        set(&pool, "test_hook", 10).await.unwrap();
        set(&pool, "test_hook", 20).await.unwrap();

        let cursor = get(&pool, "test_hook").await.unwrap();
        assert_eq!(cursor, 20);
    }

    #[tokio::test]
    async fn test_min_cursor_empty_table() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        let min = min_cursor(&pool).await.unwrap();
        assert_eq!(min, None);
    }

    #[tokio::test]
    async fn test_min_cursor_with_rows() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        set(&pool, "hook_a", 10).await.unwrap();
        set(&pool, "hook_b", 25).await.unwrap();
        set(&pool, "hook_c", 5).await.unwrap();

        let min = min_cursor(&pool).await.unwrap();
        assert_eq!(min, Some(5));
    }

    #[tokio::test]
    async fn test_get_with_flag_missing_hook() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        let (cursor, flag) = get_with_flag(&pool, "missing").await.unwrap();
        assert_eq!(cursor, 0);
        assert!(!flag);
    }

    #[tokio::test]
    async fn test_cursor_set_clears_rebootstrap_flag() {
        let (pool, _temp_dir) = db::test_utils::create_test_pool().await;

        // Manually set a cursor with the flag set
        sqlx::query(
            r#"INSERT INTO trigger_cursors (hook_name, last_processed_seq, needs_rebootstrap, updated_at)
               VALUES (?, ?, 1, datetime('now', 'subsec'))"#,
        )
        .bind("test_hook")
        .bind(0)
        .execute(&pool)
        .await
        .unwrap();

        // Verify the flag is set
        let (_, flag_before) = get_with_flag(&pool, "test_hook").await.unwrap();
        assert!(flag_before);

        // Update the cursor
        set(&pool, "test_hook", 10).await.unwrap();

        // Verify the flag is cleared
        let (cursor, flag_after) = get_with_flag(&pool, "test_hook").await.unwrap();
        assert_eq!(cursor, 10);
        assert!(!flag_after);
    }
}
