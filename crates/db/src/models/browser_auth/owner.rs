use sqlx::SqlitePool;
use uuid::Uuid;

use super::BrowserAuthError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NodeOwner {
    pub hive_user_id: Uuid,
    pub pinned_at: i64,
}

/// Pin `hive_user_id` as the node owner if unowned, otherwise compare against the existing one.
///
/// One statement, one round trip. `DO UPDATE SET hive_user_id = hive_user_id` is a deliberate
/// no-op: it changes nothing but makes RETURNING fire on the conflict path, so the statement
/// always yields the WINNING owner. Two concurrent first-authorizations cannot both pin.
///
/// Returns Ok(()) when the caller owns the node (newly pinned or already pinned), and
/// Err(BrowserAuthError::OwnerMismatch) otherwise. On mismatch NOTHING is written: pinned_at
/// and hive_user_id are untouched and NO session is revoked (rejection is side-effect free).
pub async fn pin_or_verify_owner(
    pool: &SqlitePool,
    hive_user_id: Uuid,
    now_millis: i64,
) -> Result<(), BrowserAuthError> {
    let owner = sqlx::query_as::<_, NodeOwner>(
        r#"INSERT INTO node_owner (slot, hive_user_id, pinned_at)
           VALUES (1, ?, ?)
           ON CONFLICT(slot) DO UPDATE SET hive_user_id = hive_user_id
           RETURNING hive_user_id, pinned_at"#,
    )
    .bind(hive_user_id)
    .bind(now_millis)
    .fetch_one(pool)
    .await?;

    if owner.hive_user_id == hive_user_id {
        Ok(())
    } else {
        Err(BrowserAuthError::OwnerMismatch)
    }
}

/// Read the pinned owner, or None when the node is unowned.
pub async fn get_owner(pool: &SqlitePool) -> Result<Option<NodeOwner>, sqlx::Error> {
    sqlx::query_as::<_, NodeOwner>(
        r#"SELECT hive_user_id, pinned_at FROM node_owner WHERE slot = 1"#,
    )
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_pool;
    use uuid::Uuid;

    #[tokio::test]
    async fn unowned_node_has_no_owner() {
        let (pool, _t) = create_test_pool().await;
        assert!(get_owner(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn first_subject_pins_and_same_subject_does_not_move_pinned_at() {
        let (pool, _t) = create_test_pool().await;
        let a = Uuid::new_v4();
        pin_or_verify_owner(&pool, a, 100).await.unwrap();
        pin_or_verify_owner(&pool, a, 999).await.unwrap();
        let owner = get_owner(&pool).await.unwrap().unwrap();
        assert_eq!(owner.hive_user_id, a);
        assert_eq!(
            owner.pinned_at, 100,
            "the DO UPDATE must be a genuine no-op"
        );
    }

    #[tokio::test]
    async fn different_subject_is_rejected_without_side_effects() {
        let (pool, _t) = create_test_pool().await;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        pin_or_verify_owner(&pool, a, 100).await.unwrap();
        let err = pin_or_verify_owner(&pool, b, 200).await.unwrap_err();
        assert!(matches!(err, BrowserAuthError::OwnerMismatch));
        let owner = get_owner(&pool).await.unwrap().unwrap();
        assert_eq!(owner.hive_user_id, a);
        assert_eq!(owner.pinned_at, 100);
    }

    #[tokio::test]
    async fn concurrent_first_pin_has_exactly_one_winner() {
        let (pool, _t) = create_test_pool().await;
        // create_test_pool() sets NO busy_timeout (crates/db/src/test_utils.rs:90-100), unlike
        // DBService::new(); without this the loser gets an immediate SQLITE_BUSY and the
        // outcome assertion becomes a coin-flip.
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await
            .unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let (ra, rb) = tokio::join!(
            pin_or_verify_owner(&pool, a, 100),
            pin_or_verify_owner(&pool, b, 100),
        );
        assert_eq!(ra.is_ok() as u8 + rb.is_ok() as u8, 1, "exactly one winner");
        let owner = get_owner(&pool).await.unwrap().unwrap();
        assert!(owner.hive_user_id == a || owner.hive_user_id == b);
        // The persisted state is the real proof and holds however the loser failed.
        let loser = if ra.is_ok() { b } else { a };
        assert_ne!(owner.hive_user_id, loser);
    }
}
