---
id: "004"
phase: 1
title: "Add handoff create and the single-statement atomic claim"
status: passed
depends_on: ["001","003"]
parallel: false
conflicts_with: ["003","005","022"]
files:
  - "crates/db/src/models/browser_auth/handoff.rs"
  - "crates/db/src/models/browser_auth/mod.rs"
siblings: ["crates/remote/src/db/oauth.rs","crates/remote/src/auth/handoff.rs"]
irreversible: false
scope_test: "crates/db/src/models/browser_auth/handoff.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS1"]
---
## Failing test (write first)
File: `crates/db/src/models/browser_auth/handoff.rs` — colocated `#[cfg(test)] mod tests`, `crate::test_utils::create_test_pool()`.

```rust
#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use super::*;
    use crate::test_utils::create_test_pool;

    async fn seed(pool: &sqlx::SqlitePool, id: Uuid, hash: &str, now: i64) {
        create_handoff(pool, id, "github", "verifier-abc", hash, now).await.unwrap();
    }

    #[tokio::test]
    async fn ttl_is_exactly_ten_minutes() {
        let (pool, _t) = create_test_pool().await;
        let id = Uuid::new_v4();
        let row = create_handoff(&pool, id, "github", "v", "hashA", 1_000).await.unwrap();
        assert_eq!(row.expires_at - row.created_at, 600_000);
        assert_eq!(row.expires_at, 601_000);
        assert_eq!(row.state, "pending");
    }

    #[tokio::test]
    async fn expiry_boundary_is_strictly_greater_than_now() {
        let (pool, _t) = create_test_pool().await;
        let a = Uuid::new_v4();
        seed(&pool, a, "hashA", 0).await;
        assert!(claim_handoff(&pool, a, "hashA", 599_999).await.unwrap().is_some());
        let b = Uuid::new_v4();
        seed(&pool, b, "hashA", 0).await;
        assert!(claim_handoff(&pool, b, "hashA", 600_000).await.unwrap().is_none(),
            "at exactly created_at + TTL the handoff IS expired");
        let c = Uuid::new_v4();
        seed(&pool, c, "hashA", 0).await;
        assert!(claim_handoff(&pool, c, "hashA", 600_001).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn wrong_browser_does_not_consume_the_rightful_handoff() {
        let (pool, _t) = create_test_pool().await;
        let id = Uuid::new_v4();
        seed(&pool, id, "hashA", 0).await;
        assert!(claim_handoff(&pool, id, "hashB", 1_000).await.unwrap().is_none());
        let won = claim_handoff(&pool, id, "hashA", 1_000).await.unwrap().unwrap();
        assert_eq!(won.app_verifier, "verifier-abc");
        assert_eq!(won.provider, "github");
    }

    #[tokio::test]
    async fn replay_is_rejected_and_unknown_id_is_not_an_error() {
        let (pool, _t) = create_test_pool().await;
        let id = Uuid::new_v4();
        seed(&pool, id, "hashA", 0).await;
        assert!(claim_handoff(&pool, id, "hashA", 1_000).await.unwrap().is_some());
        assert!(claim_handoff(&pool, id, "hashA", 1_000).await.unwrap().is_none());
        assert!(claim_handoff(&pool, Uuid::new_v4(), "hashA", 1_000).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn concurrent_claim_has_exactly_one_consumer() {
        let (pool, _t) = create_test_pool().await;
        sqlx::query("PRAGMA busy_timeout = 5000").execute(&pool).await.unwrap();
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
        let state: String = sqlx::query_scalar(
            "SELECT state FROM browser_oauth_handoffs WHERE handoff_id = ?")
            .bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(state, "claimed");
        assert!(claim_handoff(&pool, id, "hashA", 1_000).await.unwrap().is_none());
    }
}
```


## Change
**File:** `crates/db/src/models/browser_auth/mod.rs`
**Anchor:** the module/re-export block left by task 003.
**Before:**
```rust
mod owner;

pub use owner::{NodeOwner, get_owner, pin_or_verify_owner};
```
**After:**
```rust
mod handoff;
mod owner;

pub use handoff::{BrowserHandoff, HANDOFF_TTL_MILLIS, claim_handoff, create_handoff};
pub use owner::{NodeOwner, get_owner, pin_or_verify_owner};
```
Only these two lines change; `BrowserAuthError` is untouched.

**File:** `crates/db/src/models/browser_auth/handoff.rs` — create. Exact interface:
```rust
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
pub async fn create_handoff(pool: &SqlitePool, handoff_id: Uuid, provider: &str,
    app_verifier: &str, binding_hash: &str, now_millis: i64)
    -> Result<BrowserHandoff, sqlx::Error>;

/// Atomically claim a handoff. ONE statement -- never SELECT-then-UPDATE.
///
/// `Some(row)` means THIS caller won and owns the redemption. `None` means not claimable
/// (unknown id, wrong browser, expired, or already claimed) and, critically, that nothing was
/// consumed: a wrong-browser or expired attempt leaves a rightful pending row exactly as it was.
///
/// Claiming is TERMINAL. Redemption success and redemption failure both leave the row 'claimed',
/// so a copied or replayed callback can never mint a second session. Recovery from a failed
/// redemption is a fresh OAuth initiation, never a re-claim.
pub async fn claim_handoff(pool: &SqlitePool, handoff_id: Uuid, binding_hash: &str,
    now_millis: i64) -> Result<Option<BrowserHandoff>, sqlx::Error>;
```
SQL — runtime form, both verified in scratch sqlite3:
```sql
-- create: bind the FULL column list with 'pending' bound EXPLICITLY. The migration DEFAULT is a
-- schema-level backstop only; the explicit bind keeps the initial state visible at the one call
-- site that creates handoffs.
INSERT INTO browser_oauth_handoffs
    (handoff_id, provider, app_verifier, binding_hash, state, created_at, expires_at)
VALUES (?, ?, ?, ?, 'pending', ?, ?)
RETURNING handoff_id, provider, app_verifier, binding_hash, state, created_at, expires_at

-- claim
UPDATE browser_oauth_handoffs
   SET state = 'claimed'
 WHERE handoff_id = ?
   AND state = 'pending'
   AND expires_at > ?
   AND binding_hash = ?
RETURNING handoff_id, provider, app_verifier, binding_hash, state, created_at, expires_at
```

**Sibling alignment (rubric 9).** Read `crates/remote/src/db/oauth.rs:206-280` before writing: `mark_redeemed` is the house single-consumer idiom (`UPDATE ... WHERE id = $1 AND status = 'authorized'`, then `rows_affected() == 0 -> AlreadyRedeemed`). It is Postgres/macro-form — copy the SHAPE, not the syntax. Also read `crates/remote/src/auth/handoff.rs:31-34` for the TTL this must match, and `browser_auth/owner.rs` (task 003) for the module's runtime-query and test conventions.

**Symbol grounding:** This task introduces `create_handoff()`, `claim_handoff()`, the `HANDOFF_TTL_MILLIS` constant and the `BrowserHandoff` type, and is the sole reader and writer of the `browser_oauth_handoffs()` table created by task 001. `BrowserAuthError` is defined by task 003; this task only re-exports alongside it.


## Allowed moves
[
  "Add exactly `mod handoff;` and one `pub use handoff::{...}` line to browser_auth/mod.rs.",
  "Create browser_auth/handoff.rs with the const, struct, two functions and colocated tests.",
  "Runtime-checked query forms only; the claim must be ONE statement."
]


## STOP triggers
[
  "Any read (SELECT / fetch_optional) executed BEFORE the claim UPDATE — that reintroduces wrong-browser consumption and breaks the isolation tests.",
  "Adding a 'failed' or 'completed' state, or any transition out of 'claimed' — the CHECK constraint from 001 permits only pending/claimed, and terminal-on-claim is what makes replay unrepresentable.",
  "Hashing app_verifier — the daemon must replay it verbatim to Hive.",
  "Using DELETE as the consumption mechanism — deletion is indistinguishable from 'never existed' and loses the replay evidence.",
  "Any macro-form sqlx query, or any hand-written CREATE TABLE in tests.",
  "Editing browser_auth/owner.rs or crates/db/src/models/mod.rs — those belong to task 003."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p db browser_auth::handoff" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 004` exits 0.
2. `cargo test -p db browser_auth` — owner + handoff tests green; run 3x for flake.
3. TS1 ownership check: confirm the five behaviours named in TS1 that belong to handoffs (exact expiry, wrong-browser non-consumption, concurrent single claim, replay rejection, hash-only persistence) each have a named test above, and that clock values are passed explicitly (no `Utc::now()` in this file).


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 004` exits 0
