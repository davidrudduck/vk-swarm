---
id: "005"
phase: 1
title: "Add session create, authenticate-by-hash, revoke-one and revoke-all"
status: passed
depends_on: ["001","003","004"]
parallel: false
conflicts_with: ["003","004","022"]
files:
  - "crates/db/src/models/browser_auth/session.rs"
  - "crates/db/src/models/browser_auth/mod.rs"
siblings: ["crates/remote/src/db/auth.rs"]
irreversible: false
scope_test: "crates/db/src/models/browser_auth/session.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `crates/db/src/models/browser_auth/session.rs` — colocated `#[cfg(test)] mod tests`, `crate::test_utils::create_test_pool()`.

```rust
#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use super::*;
    use crate::test_utils::create_test_pool;

    #[tokio::test]
    async fn create_then_authenticate_round_trips() {
        let (pool, _t) = create_test_pool().await;
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 42).await.unwrap();
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
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 1).await.unwrap();
        create_session(&pool, Uuid::new_v4(), "hashB", owner, 1).await.unwrap();
        assert!(revoke_session(&pool, "hashA", 10).await.unwrap());
        assert!(authenticate_session(&pool, "hashA").await.unwrap().is_none());
        assert!(authenticate_session(&pool, "hashB").await.unwrap().is_some(),
            "SC7: other browsers must survive one browser's logout");
    }

    #[tokio::test]
    async fn revoke_is_idempotent_and_does_not_rewrite_the_timestamp() {
        let (pool, _t) = create_test_pool().await;
        create_session(&pool, Uuid::new_v4(), "hashA", Uuid::new_v4(), 1).await.unwrap();
        assert!(revoke_session(&pool, "hashA", 10).await.unwrap());
        assert!(!revoke_session(&pool, "hashA", 99).await.unwrap());
        let at: Option<i64> = sqlx::query_scalar(
            "SELECT revoked_at FROM browser_sessions WHERE token_hash = ?")
            .bind("hashA").fetch_one(&pool).await.unwrap();
        assert_eq!(at, Some(10));
    }

    #[tokio::test]
    async fn revoke_all_counts_only_live_sessions() {
        let (pool, _t) = create_test_pool().await;
        assert_eq!(revoke_all_sessions(&pool, 5).await.unwrap(), 0);
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 1).await.unwrap();
        create_session(&pool, Uuid::new_v4(), "hashB", owner, 1).await.unwrap();
        revoke_session(&pool, "hashB", 2).await.unwrap();
        assert_eq!(revoke_all_sessions(&pool, 5).await.unwrap(), 1);
        assert!(authenticate_session(&pool, "hashA").await.unwrap().is_none());
        assert!(authenticate_session(&pool, "hashB").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn token_hash_is_unique() {
        let (pool, _t) = create_test_pool().await;
        let owner = Uuid::new_v4();
        create_session(&pool, Uuid::new_v4(), "hashA", owner, 1).await.unwrap();
        assert!(create_session(&pool, Uuid::new_v4(), "hashA", owner, 1).await.is_err(),
            "one presented token must never resolve to two session rows");
    }
}
```
Note the contract asserted by ABSENCE: `authenticate_session` takes no time argument, so no elapsed-time predicate can exist. Do NOT add a "raw token is not in the table" test — `create_session` takes the hash as a parameter, so such a test could only fail if the TEST passed the raw string.


## Change
**File:** `crates/db/src/models/browser_auth/mod.rs`
**Anchor:** the module/re-export block as left by tasks 003 and 004.
**Before:**
```rust
mod handoff;
mod owner;
```
**After:**
```rust
mod handoff;
mod owner;
mod session;
```
and append one re-export line after the existing `pub use owner::{...};`:
```rust
pub use session::{BrowserSession, authenticate_session, create_session, revoke_all_sessions, revoke_session};
```

**File:** `crates/db/src/models/browser_auth/session.rs` — create. Exact interface:
```rust
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

pub async fn create_session(pool: &SqlitePool, id: Uuid, token_hash: &str, hive_user_id: Uuid,
    now_millis: i64) -> Result<BrowserSession, sqlx::Error>;

/// The authorization primitive the protected router binds to.
///
/// Returns the live session for this token hash, or None when the hash is unknown OR the session
/// is revoked. There is deliberately NO time argument and NO expiry predicate: validity is
/// revocation state only, never elapsed time and never Hive availability (D6/D9). A Hive outage
/// cannot make this return None.
pub async fn authenticate_session(pool: &SqlitePool, token_hash: &str)
    -> Result<Option<BrowserSession>, sqlx::Error>;

/// Revoke ONLY the presenting browser's session (keyed by the hash of the token it presented).
/// True when a live session was revoked, false when unknown or already revoked (idempotent).
/// Other browsers' sessions, the pinned owner and daemon Hive credentials are untouched.
pub async fn revoke_session(pool: &SqlitePool, token_hash: &str, now_millis: i64)
    -> Result<bool, sqlx::Error>;

/// Revoke every live session; returns the number revoked.
///
/// This is the FIRST step of explicit Hive disconnect. The rest of that sequence -- stop sync,
/// then delete daemon credentials, surfacing any deletion failure, retaining the pinned owner --
/// belongs to the caller (O8: SQLite revocation and Keychain/file deletion cannot share one
/// transaction). Do not implement it here and do not touch node_owner here.
pub async fn revoke_all_sessions(pool: &SqlitePool, now_millis: i64) -> Result<u64, sqlx::Error>;
```
SQL — runtime form:
```sql
-- authenticate
SELECT id, token_hash, hive_user_id, created_at, revoked_at
  FROM browser_sessions WHERE token_hash = ? AND revoked_at IS NULL

-- revoke one   (rows_affected() == 1 -> true)
UPDATE browser_sessions SET revoked_at = ? WHERE token_hash = ? AND revoked_at IS NULL

-- revoke all   (return rows_affected())
UPDATE browser_sessions SET revoked_at = ? WHERE revoked_at IS NULL
```

**Sibling alignment (rubric 9).** Read `crates/remote/src/db/auth.rs:180-246` first: `revoke` / `revoke_all_user_sessions` are the source of the `AND revoked_at IS NULL` idempotent-revoke guard, which also prevents a later revoke from overwriting the original revocation timestamp. Match that guard exactly.

**Symbol grounding:** This task introduces `create_session()`, `authenticate_session()`, `revoke_session()`, `revoke_all_sessions()` and the `BrowserSession` type, and is the sole reader and writer of the `browser_sessions()` table created by task 001.


## Allowed moves
[
  "Add exactly `mod session;` and one `pub use session::{...}` line to browser_auth/mod.rs.",
  "Create browser_auth/session.rs with the struct, four functions and colocated tests.",
  "Runtime-checked query forms only."
]


## STOP triggers
[
  "Any expires_at column reference or elapsed-time predicate creeping into session authentication.",
  "revoke_all_sessions touching node_owner, credentials or sync — the owner must survive disconnect (D4) and credential deletion is the caller's step.",
  "Keying revoke_session by session id instead of token_hash — the presenting browser holds a token, not an id, and an id-keyed API invites revoking someone else's session.",
  "Any DELETE FROM browser_sessions used as revocation — revocation must be observable as state.",
  "Editing browser_auth/owner.rs or handoff.rs — they belong to tasks 003/004."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p db browser_auth::session" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 005` exits 0.
2. `cargo test -p db browser_auth` — owner + handoff + session all green.
3. `git grep -n 'expires_at' crates/db/src/models/browser_auth/session.rs` returns nothing.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 005` exits 0
