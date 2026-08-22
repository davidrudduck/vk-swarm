---
id: "003"
phase: 1
title: "Add the browser_auth model with an atomic owner pin-or-compare"
status: passed
depends_on: ["001"]
parallel: false
conflicts_with: ["004","005","022"]
files:
  - "crates/db/src/models/browser_auth/mod.rs"
  - "crates/db/src/models/browser_auth/owner.rs"
  - "crates/db/src/models/mod.rs"
siblings: ["crates/db/src/models/trigger_cursor.rs","crates/db/src/models/event_journal/queries.rs","crates/db/src/models/node_outbox.rs"]
irreversible: false
scope_test: "crates/db/src/models/browser_auth/owner.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `crates/db/src/models/browser_auth/owner.rs` — colocated `#[cfg(test)] mod tests` using `crate::test_utils::create_test_pool()` (never hand-written schema).

```rust
#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use super::*;
    use crate::test_utils::create_test_pool;

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
        assert_eq!(owner.pinned_at, 100, "the DO UPDATE must be a genuine no-op");
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
        sqlx::query("PRAGMA busy_timeout = 5000").execute(&pool).await.unwrap();
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
```


## Change
**File:** `crates/db/src/models/mod.rs`
**Anchor:** the alphabetical `pub mod` block (verified L17-21).
**Before:**
```rust
pub mod all_tasks;
pub mod dashboard;
```
**After:**
```rust
pub mod all_tasks;
pub mod browser_auth;
pub mod dashboard;
```

**File:** `crates/db/src/models/browser_auth/mod.rs` — create.
**After:**
```rust
//! Local browser-authorization records: the pinned Hive owner, browser-bound OAuth handoffs,
//! and hashed opaque browser sessions.
//!
//! Every timestamp in this module is unix-epoch MILLISECONDS supplied by the caller
//! (`now_millis`), never `datetime('now')`. See 20260821000000_add_browser_auth.sql for why.
//!
//! This module never generates or hashes a secret. Callers pass pre-computed `token_hash` /
//! `binding_hash` values (lowercase hex SHA-256), which keeps `crates/db` free of crypto/RNG
//! dependencies and keeps the hashing seam in one place
//! (`crates/server/src/auth/seams.rs::hash_token`).

mod owner;

pub use owner::{NodeOwner, get_owner, pin_or_verify_owner};

/// Errors that are not plain database failures.
#[derive(Debug, thiserror::Error)]
pub enum BrowserAuthError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    /// A different Hive subject attempted to authorize an already-owned node.
    #[error("node is owned by a different hive subject")]
    OwnerMismatch,
}
```

**File:** `crates/db/src/models/browser_auth/owner.rs` — create. Exact interface:
```rust
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
pub async fn pin_or_verify_owner(pool: &SqlitePool, hive_user_id: Uuid, now_millis: i64)
    -> Result<(), BrowserAuthError>;

/// Read the pinned owner, or None when the node is unowned.
pub async fn get_owner(pool: &SqlitePool) -> Result<Option<NodeOwner>, sqlx::Error>;
```
SQL for `pin_or_verify_owner` — RUNTIME form (`sqlx::query_as::<_, NodeOwner>(...)`), verified in a
scratch sqlite3 to return the EXISTING owner on the conflict path:
```sql
INSERT INTO node_owner (slot, hive_user_id, pinned_at)
VALUES (1, ?, ?)
ON CONFLICT(slot) DO UPDATE SET hive_user_id = hive_user_id
RETURNING hive_user_id, pinned_at
```
Compare the returned `hive_user_id` with the candidate; unequal -> `OwnerMismatch`.

**Sibling alignment (rubric 9).** Before writing, read `crates/db/src/models/trigger_cursor.rs` (minimal single-file model; UPSERT with a deliberately narrow SET list plus a comment saying what is intentionally NOT updated — directly analogous to the no-op owner upsert), `crates/db/src/models/event_journal/queries.rs` (100% runtime-form queries; the newest additive table) and `crates/db/src/models/node_outbox.rs` (BLOB-UUID bind idiom). Match their runtime-query form and their colocated-test style. Do NOT copy `node_outbox::has_unacked_for_entity`, which is the one macro-form query in the neighbourhood.

**Symbol grounding:** This task introduces `pin_or_verify_owner()`, `get_owner()` and the `NodeOwner` / `BrowserAuthError` types, and is the sole reader and writer of the `node_owner()` table created by task 001. `create_test_pool()` is a pre-existing helper in `crates/db/src/test_utils.rs`.

**SQLite timeout clarification.** sqlx 0.8.6 already installs a 5-second busy timeout on every SQLite connection, including `create_test_pool()`. The explicit `PRAGMA busy_timeout = 5000` in the concurrency test is belt-and-braces only; do not add Codex C6/C7 pool hooks or connection-specific timeout changes. Preserve the exact one-winner and persisted-row race assertions.



## Allowed moves
[
  "Insert exactly one `pub mod browser_auth;` line into crates/db/src/models/mod.rs, alphabetically.",
  "Create browser_auth/mod.rs with only the doc comment, `mod owner;`, the owner re-exports and BrowserAuthError.",
  "Create browser_auth/owner.rs with the two functions, NodeOwner, and the colocated tests.",
  "Runtime-checked query forms only. No sqlx::query!/query_as!/query_scalar! anywhere in this task."
]


## STOP triggers
[
  "Any SELECT-then-INSERT/UPDATE shape — it is racy and defeats the concurrency test. If a read-before-write ever becomes unavoidable it must use BEGIN IMMEDIATE (see event_journal::compact for the SQLITE_BUSY_SNAPSHOT rationale).",
  "Any function that replaces, clears or resets the owner — owner reset is out of scope, and any internal owner-replacement op would owe a same-transaction revoke-all.",
  "`pinned_at` changing on the already-owned path.",
  "Any macro-form sqlx query (query!/query_as!/query_scalar!) — .sqlx has no cache for these new tables and offline builds would break.",
  "Any `#[derive(TS)]` or urge to run `npm run generate-types` — these are internal persistence types that never cross the API boundary.",
  "Any hand-written CREATE TABLE in the test module."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p db browser_auth::owner" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 003` exits 0.
2. `cargo test -p db browser_auth::owner` — 4 tests green; run it 3x to confirm the concurrency test is not flaky.
3. `git grep -n 'query_as!\|query!\|query_scalar!' crates/db/src/models/browser_auth/` returns nothing.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 003` exits 0
