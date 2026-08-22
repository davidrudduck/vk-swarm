use sqlx::SqlitePool;
use uuid::Uuid;

/// Exactly ten minutes, per spec. A handoff is claimable while `expires_at > now_millis`, so at
/// exactly created_at + HANDOFF_TTL_MILLIS it is ALREADY expired. This matches the Hive-side
/// `const HANDOFF_TTL: i64 = 10; // minutes` in crates/remote/src/auth/handoff.rs:31-34.
pub const HANDOFF_TTL_MILLIS: i64 = 600_000;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrowserHandoff {
    pub handoff_id: Uuid,
    pub provider: String,
    /// The verifier the daemon presents to Hive at redemption. Stored RAW, not hashed -- it must
    /// be replayable by the daemon. It is never returned to the browser.
    pub app_verifier: String,
    /// Lowercase hex SHA-256 of the pre-auth binding cookie value. The raw value never reaches
    /// the database.
    pub binding_hash: String,
    pub state: String, // 'pending' | 'claimed'
    pub created_at: i64,
    pub expires_at: i64,
}

/// Record a pending handoff. `expires_at` is computed as `now_millis + HANDOFF_TTL_MILLIS` HERE,
/// so the TTL cannot drift between call sites.
pub async fn create_handoff(
    pool: &SqlitePool,
    handoff_id: Uuid,
    provider: &str,
    app_verifier: &str,
    binding_hash: &str,
    now_millis: i64,
) -> Result<BrowserHandoff, sqlx::Error> {
    let expires_at = now_millis + HANDOFF_TTL_MILLIS;

    sqlx::query_as::<_, BrowserHandoff>(
        r#"INSERT INTO browser_oauth_handoffs
            (handoff_id, provider, app_verifier, binding_hash, state, created_at, expires_at)
        VALUES (?, ?, ?, ?, 'pending', ?, ?)
        RETURNING handoff_id, provider, app_verifier, binding_hash, state, created_at, expires_at"#,
    )
    .bind(handoff_id)
    .bind(provider)
    .bind(app_verifier)
    .bind(binding_hash)
    .bind(now_millis)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

/// Atomically claim a handoff. ONE statement -- never SELECT-then-UPDATE.
///
/// `Some(row)` means THIS caller won and owns the redemption. `None` means not claimable
/// (unknown id, wrong browser, expired, or already claimed) and, critically, that nothing was
/// consumed: a wrong-browser or expired attempt leaves a rightful pending row exactly as it was.
///
/// Claiming is TERMINAL. Redemption success and redemption failure both leave the row 'claimed',
/// so a copied or replayed callback can never mint a second session. Recovery from a failed
/// redemption is a fresh OAuth initiation, never a re-claim.
pub async fn claim_handoff(
    pool: &SqlitePool,
    handoff_id: Uuid,
    binding_hash: &str,
    now_millis: i64,
) -> Result<Option<BrowserHandoff>, sqlx::Error> {
    sqlx::query_as::<_, BrowserHandoff>(
        r#"UPDATE browser_oauth_handoffs
           SET state = 'claimed'
         WHERE handoff_id = ?
           AND state = 'pending'
           AND expires_at > ?
           AND binding_hash = ?
        RETURNING handoff_id, provider, app_verifier, binding_hash, state, created_at, expires_at"#,
    )
    .bind(handoff_id)
    .bind(now_millis)
    .bind(binding_hash)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_pool;
    use uuid::Uuid;

    async fn seed(pool: &sqlx::SqlitePool, id: Uuid, hash: &str, now: i64) {
        create_handoff(pool, id, "github", "verifier-abc", hash, now)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ttl_is_exactly_ten_minutes() {
        let (pool, _t) = create_test_pool().await;
        let id = Uuid::new_v4();
        let row = create_handoff(&pool, id, "github", "v", "hashA", 1_000)
            .await
            .unwrap();
        assert_eq!(row.expires_at - row.created_at, 600_000);
        assert_eq!(row.expires_at, 601_000);
        assert_eq!(row.state, "pending");
    }

    #[tokio::test]
    async fn expiry_boundary_is_strictly_greater_than_now() {
        let (pool, _t) = create_test_pool().await;
        let a = Uuid::new_v4();
        seed(&pool, a, "hashA", 0).await;
        assert!(
            claim_handoff(&pool, a, "hashA", 599_999)
                .await
                .unwrap()
                .is_some()
        );
        let b = Uuid::new_v4();
        seed(&pool, b, "hashA", 0).await;
        assert!(
            claim_handoff(&pool, b, "hashA", 600_000)
                .await
                .unwrap()
                .is_none(),
            "at exactly created_at + TTL the handoff IS expired"
        );
        let c = Uuid::new_v4();
        seed(&pool, c, "hashA", 0).await;
        assert!(
            claim_handoff(&pool, c, "hashA", 600_001)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn wrong_browser_does_not_consume_the_rightful_handoff() {
        let (pool, _t) = create_test_pool().await;
        let id = Uuid::new_v4();
        seed(&pool, id, "hashA", 0).await;
        assert!(
            claim_handoff(&pool, id, "hashB", 1_000)
                .await
                .unwrap()
                .is_none()
        );
        let won = claim_handoff(&pool, id, "hashA", 1_000)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(won.app_verifier, "verifier-abc");
        assert_eq!(won.provider, "github");
    }

    #[tokio::test]
    async fn replay_is_rejected_and_unknown_id_is_not_an_error() {
        let (pool, _t) = create_test_pool().await;
        let id = Uuid::new_v4();
        seed(&pool, id, "hashA", 0).await;
        assert!(
            claim_handoff(&pool, id, "hashA", 1_000)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            claim_handoff(&pool, id, "hashA", 1_000)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            claim_handoff(&pool, Uuid::new_v4(), "hashA", 1_000)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn concurrent_claim_has_exactly_one_consumer() {
        let (pool, _t) = create_test_pool().await;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await
            .unwrap();
        let id = Uuid::new_v4();
        seed(&pool, id, "hashA", 0).await;
        let (r1, r2) = tokio::join!(
            claim_handoff(&pool, id, "hashA", 1_000),
            claim_handoff(&pool, id, "hashA", 1_000),
        );
        let wins = r1.as_ref().ok().map_or(0, |o| o.is_some() as u8)
            + r2.as_ref().ok().map_or(0, |o| o.is_some() as u8);
        assert_eq!(wins, 1, "exactly one claimant may win");
        // Persisted state is the real proof, however the loser failed.
        let state: String =
            sqlx::query_scalar("SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(state, "claimed");
        assert!(
            claim_handoff(&pool, id, "hashA", 1_000)
                .await
                .unwrap()
                .is_none()
        );
    }
}
