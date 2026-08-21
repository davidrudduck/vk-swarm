---
id: "001"
phase: 1
title: "Add the additive browser-auth migration: node_owner, browser_oauth_handoffs, browser_sessions"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - "crates/db/migrations/20260821000000_add_browser_auth.sql"
  - "crates/db/src/test_utils.rs"
siblings: ["crates/db/migrations/20260812000000_add_event_journal.sql","crates/db/migrations/20260201000400_add_node_outbox.sql","crates/db/migrations/20250617183714_init.sql","crates/db/migrations/20250620212427_execution_processes.sql","crates/db/migrations/20250620214100_remove_stdout_stderr_from_task_attempts.sql"]
irreversible: true
scope_test: "crates/db/src/test_utils.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: crates/db/src/test_utils.rs — append inside the EXISTING `#[cfg(test)] mod tests` block (the one that already contains `test_create_test_pool`, ~L133).

```rust
#[tokio::test]
async fn browser_auth_migration_creates_owner_handoff_and_session_tables() {
    let (pool, _tmp) = create_test_pool().await;

    for table in ["node_owner", "browser_oauth_handoffs", "browser_sessions"] {
        let found: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        .expect("sqlite_master query failed");
        assert_eq!(found.as_deref(), Some(table), "migration did not create {table}");
    }

    // The singleton is STRUCTURAL, not a convention: slot is CHECK-pinned to 1.
    let second_slot = sqlx::query(
        "INSERT INTO node_owner (slot, hive_user_id, pinned_at) VALUES (2, x'aa', 1)",
    )
    .execute(&pool)
    .await;
    assert!(second_slot.is_err(), "node_owner accepted a second slot");

    // Sessions are revocation-state only (D9/SC5) — a time-based expiry column must NOT exist.
    let cols: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('browser_sessions')")
        .fetch_all(&pool)
        .await
        .expect("pragma_table_info failed");
    assert!(!cols.iter().any(|c| c == "expires_at"),
        "browser_sessions must have no expiry column; got {cols:?}");
    assert!(cols.iter().any(|c| c == "revoked_at"), "missing revoked_at; got {cols:?}");
}
```

RED before the migration file exists: `create_test_pool()` builds its template from the embedded `sqlx::migrate!("./migrations")`, so the three tables are simply absent and the first assert fails.


## Change
**File:** `crates/db/migrations/20260821000000_add_browser_auth.sql`
**Anchor:** new file. Verified: the highest existing migration is `20260812000000_add_event_journal.sql` (`ls crates/db/migrations/`), so this prefix is strictly greater and unused.
**Before:** (file does not exist)
**After:** exactly this SQL, header comment included —

```sql
-- Local browser-authorization schema (local-node-browser-oauth). Three additive tables; no
-- existing table is altered.
--
-- Divergence from every sibling table in this directory: all timestamps here are INTEGER
-- unix-epoch MILLISECONDS bound explicitly by the caller, not TEXT `datetime('now','subsec')`
-- defaults. Two reasons. (1) Handoff expiry is a stored-vs-bound comparison, and the
-- event_journal compaction regression (20260812000000, see
-- compact_keeps_same_day_rows_inside_the_retention_window) proved that comparing a TEXT
-- datetime() column against an RFC-3339 bind collates wrong ('T' > ' '). Here that failure mode
-- fails OPEN: an expired handoff would still be claimable. (2) A SQL DEFAULT bypasses the
-- injected test clock, and exact 10-minute expiry must be driven deterministically (TS1).

-- Exactly one owner. `slot` is pinned to 1 by CHECK, which makes the singleton structural.
-- First writer wins: the pin-or-compare upsert uses a no-op DO UPDATE so RETURNING yields the
-- EXISTING owner on conflict without replacing it.
CREATE TABLE IF NOT EXISTS node_owner (
    slot         INTEGER PRIMARY KEY CHECK (slot = 1),
    hive_user_id BLOB    NOT NULL,   -- UUID, stable Hive subject (ProfileResponse.user_id)
    pinned_at    INTEGER NOT NULL    -- unix epoch millis
);

-- Browser-bound OAuth handoffs. `binding_hash` is the SHA-256 hex of the pre-auth browser
-- cookie value; the raw value never reaches this table. `app_verifier` IS stored raw -- it is
-- the verifier the daemon must present to Hive at redemption, so it cannot be hashed. `state`
-- is terminal after 'claimed': redemption success AND redemption failure both leave the row
-- unclaimable, so replay can never mint a second session.
CREATE TABLE IF NOT EXISTS browser_oauth_handoffs (
    handoff_id   BLOB    PRIMARY KEY,  -- UUID issued by Hive
    provider     TEXT    NOT NULL,
    app_verifier TEXT    NOT NULL,
    binding_hash TEXT    NOT NULL,     -- lowercase hex SHA-256, 64 chars
    state        TEXT    NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'claimed')),
    created_at   INTEGER NOT NULL,     -- unix epoch millis
    expires_at   INTEGER NOT NULL      -- unix epoch millis; claimable while expires_at > now
);

-- Opaque browser sessions. Only the SHA-256 hex of the 256-bit base64url token is stored; the
-- raw token exists only in the Set-Cookie header and the presenting browser. There is
-- deliberately NO expiry column: authorization is revocation-state only, never time-based
-- (D9/SC5).
CREATE TABLE IF NOT EXISTS browser_sessions (
    id           BLOB    PRIMARY KEY,  -- UUID v4
    token_hash   TEXT    NOT NULL UNIQUE,
    hive_user_id BLOB    NOT NULL,     -- the pinned owner subject
    created_at   INTEGER NOT NULL,     -- unix epoch millis
    revoked_at   INTEGER              -- NULL while live
);

-- Authentication is a point lookup on the hash for every protected request; the UNIQUE
-- constraint above already provides that index, so no extra index is created here.
```

**File:** `crates/db/src/test_utils.rs`
**Anchor:** the existing `#[cfg(test)] mod tests { use super::*; #[tokio::test] async fn test_create_test_pool() ... }` block at the end of the file (~L133).
**Before:** the block ends with the closing `}` of `test_create_test_pool`.
**After:** the same block with the new `browser_auth_migration_creates_owner_handoff_and_session_tables` test appended after `test_create_test_pool`. Do not modify `create_test_pool` or `create_test_pool_with_migrations` themselves.

**IRREVERSIBLE — sidecar decision point DP1, halt code `human_gate_required`.** This task applies an irreversible additive production schema migration. The executor MUST record the human approval token (`docs/plans/local-node-browser-oauth/reviews/001.approved`) BEFORE running the task; the gate checks for it. Wire the halt code, do not merely label the task irreversible.

**Symbol grounding:** This task introduces the three tables `node_owner()`, `browser_oauth_handoffs()` and `browser_sessions()` — the grounding markers here are cosmetic, these are SQL table names created by the migration file, not callable functions. It also introduces the test `browser_auth_migration_creates_owner_handoff_and_session_tables()` in `crates/db/src/test_utils.rs`. It introduces no Rust API; the model functions that read and write these tables arrive in tasks 003, 004 and 005.


## Allowed moves
[
  "Create exactly the one migration file named above, with exactly the SQL given (comments included).",
  "Append exactly one `#[tokio::test]` to the EXISTING `#[cfg(test)] mod tests` block in crates/db/src/test_utils.rs.",
  "Nothing else. No ALTER TABLE, no change to any existing migration, no change to the two pool constructors."
]


## STOP triggers
[
  "A migration file with version >= 20260821000000 already exists (another slice landed first) — STOP and re-derive the version.",
  "DP1: the human approval token docs/plans/local-node-browser-oauth/reviews/001.approved is absent — HALT with `human_gate_required`; do not apply the migration.",
  "Any urge to ALTER TABLE or otherwise touch an existing table — this task is additive-only by definition.",
  "Any urge to add DEFAULT (datetime(...)) or DEFAULT CURRENT_TIMESTAMP to a timestamp column — it bypasses the injected clock and reintroduces the event_journal collation bug.",
  "Any urge to add an `expires_at` column to browser_sessions — session validity is revocation-state only.",
  "`create_test_pool()` starts failing for unrelated existing db tests → the migration is not additive; STOP.",
  "Any urge to hand-write CREATE TABLE inside a test helper — forbidden by CLAUDE.md; the migration is the only schema source."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
Record in the decisions ledger:
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p db test_utils" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 001` exits 0.
2. `cargo test -p db test_utils` — both tests green.
3. `cargo test -p db` — no pre-existing db test regressed (proves additivity).
4. DP1 evidence: paste the path and timestamp of the recorded human approval token.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 001` exits 0
