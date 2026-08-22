use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrowserSession {
    pub id: Uuid,
    /// Lowercase hex SHA-256 of the opaque 256-bit base64url token. The raw token is never
    /// stored, never logged, and never appears in an API body.
    pub token_hash: String,
    pub hive_user_id: Uuid,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

pub async fn create_session(
    pool: &SqlitePool,
    id: Uuid,
    token_hash: &str,
    hive_user_id: Uuid,
    now_millis: i64,
) -> Result<BrowserSession, sqlx::Error> {
    sqlx::query_as::<_, BrowserSession>(
        r#"INSERT INTO browser_sessions (id, token_hash, hive_user_id, created_at, revoked_at)
           VALUES (?, ?, ?, ?, NULL)
           RETURNING id, token_hash, hive_user_id, created_at, revoked_at"#,
    )
    .bind(id)
    .bind(token_hash)
    .bind(hive_user_id)
    .bind(now_millis)
    .fetch_one(pool)
    .await
}

/// The authorization primitive the protected router binds to.
///
/// Returns the live session for this token hash, or None when the hash is unknown OR the session
/// is revoked. There is deliberately NO time argument and NO expiry predicate: validity is
/// revocation state only, never elapsed time and never Hive availability (D6/D9). A Hive outage
/// cannot make this return None.
pub async fn authenticate_session(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<BrowserSession>, sqlx::Error> {
    sqlx::query_as::<_, BrowserSession>(
        r#"SELECT id, token_hash, hive_user_id, created_at, revoked_at
           FROM browser_sessions WHERE token_hash = ? AND revoked_at IS NULL"#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

/// Revoke ONLY the presenting browser's session (keyed by the hash of the token it presented).
/// True when a live session was revoked, false when unknown or already revoked (idempotent).
/// Other browsers' sessions, the pinned owner and daemon Hive credentials are untouched.
pub async fn revoke_session(
    pool: &SqlitePool,
    token_hash: &str,
    now_millis: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"UPDATE browser_sessions SET revoked_at = ? WHERE token_hash = ? AND revoked_at IS NULL"#,
    )
    .bind(now_millis)
    .bind(token_hash)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

/// Revoke every live session; returns the number revoked.
///
/// This is the FIRST step of explicit Hive disconnect. The rest of that sequence -- stop sync,
/// then delete daemon credentials, surfacing any deletion failure, retaining the pinned owner --
/// belongs to the caller (O8: SQLite revocation and Keychain/file deletion cannot share one
/// transaction). Do not implement it here and do not touch node_owner here.
pub async fn revoke_all_sessions(pool: &SqlitePool, now_millis: i64) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query(r#"UPDATE browser_sessions SET revoked_at = ? WHERE revoked_at IS NULL"#)
            .bind(now_millis)
            .execute(pool)
            .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_pool;
    use uuid::Uuid;

    #[tokio::test]
    async fn create_then_authenticate_round_trips() {
        let (pool, _t) = create_test_pool().await;
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 42)
            .await
            .unwrap();
        let s = authenticate_session(&pool, "hashA").await.unwrap().unwrap();
        assert_eq!(s.hive_user_id, owner);
        assert_eq!(s.created_at, 42);
        assert!(s.revoked_at.is_none());
        assert!(authenticate_session(&pool, "nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn revoke_session_is_scoped_to_the_presenting_browser() {
        let (pool, _t) = create_test_pool().await;
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 1)
            .await
            .unwrap();
        create_session(&pool, Uuid::new_v4(), "hashB", owner, 1)
            .await
            .unwrap();
        assert!(revoke_session(&pool, "hashA", 10).await.unwrap());
        assert!(
            authenticate_session(&pool, "hashA")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            authenticate_session(&pool, "hashB")
                .await
                .unwrap()
                .is_some(),
            "SC7: other browsers must survive one browser's logout"
        );
    }

    #[tokio::test]
    async fn revoke_is_idempotent_and_does_not_rewrite_the_timestamp() {
        let (pool, _t) = create_test_pool().await;
        create_session(&pool, Uuid::new_v4(), "hashA", Uuid::new_v4(), 1)
            .await
            .unwrap();
        assert!(revoke_session(&pool, "hashA", 10).await.unwrap());
        assert!(!revoke_session(&pool, "hashA", 99).await.unwrap());
        let at: Option<i64> =
            sqlx::query_scalar("SELECT revoked_at FROM browser_sessions WHERE token_hash = ?")
                .bind("hashA")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(at, Some(10));
    }

    #[tokio::test]
    async fn revoke_all_counts_only_live_sessions() {
        let (pool, _t) = create_test_pool().await;
        assert_eq!(revoke_all_sessions(&pool, 5).await.unwrap(), 0);
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 1)
            .await
            .unwrap();
        create_session(&pool, Uuid::new_v4(), "hashB", owner, 1)
            .await
            .unwrap();
        revoke_session(&pool, "hashB", 2).await.unwrap();
        assert_eq!(revoke_all_sessions(&pool, 5).await.unwrap(), 1);
        assert!(
            authenticate_session(&pool, "hashA")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            authenticate_session(&pool, "hashB")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn token_hash_is_unique() {
        let (pool, _t) = create_test_pool().await;
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 1)
            .await
            .unwrap();
        assert!(
            create_session(&pool, Uuid::new_v4(), "hashA", owner, 1)
                .await
                .is_err(),
            "one presented token must never resolve to two session rows"
        );
    }
}
