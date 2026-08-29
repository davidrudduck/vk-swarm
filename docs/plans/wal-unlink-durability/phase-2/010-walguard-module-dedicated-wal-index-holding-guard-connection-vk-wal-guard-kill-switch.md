---
id: "010"
phase: 2
title: "WalGuard module: dedicated wal-index-holding guard connection + VK_WAL_GUARD kill-switch"
status: ready
depends_on: ["002"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/wal_guard.rs"
  - "crates/db/src/lib.rs"
irreversible: false
scope_test: "crates/db/src/wal_guard.rs"
allowed_change: mixed
covers_criteria: ["SC1"]
covers_tests: ["TS3"]
---
## Failing test (write first)
Write this test FIRST in `#[cfg(test)] mod tests` inside wal_guard.rs and watch it fail (module does not exist yet): 

```rust
#[tokio::test]
async fn guard_blocks_external_unlink_hold_read() {
    if std::process::Command::new("sqlite3").arg("--version").output().is_err() {
        eprintln!("Skipping test: sqlite3 CLI not available");
        return;
    }
    let (pool, tmp) = crate::test_utils::create_test_pool().await;
    let db_path = tmp.path().join("test.db");
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'guard-probe', '/tmp/guard-probe-uniq')").execute(&pool).await.unwrap();
    let mode = ledger_mode(); // see change text — the mode task 002 recorded
    let guard = WalGuard::connect(&db_path, mode).await.unwrap();
    // 2026-08-30 vector amendment: the unlink trigger is an external WRITE session, not a read.
    let out = std::process::Command::new("sqlite3").arg(&db_path).arg("PRAGMA user_version=1;").output().unwrap();
    assert!(out.status.success());
    let wal = std::path::PathBuf::from(format!("{}-wal", db_path.display()));
    assert!(wal.exists(), "external write-session close unlinked the WAL despite the guard");
    drop(guard);
    pool.close().await;
    // Durability must survive a FULL close: a fresh offline connection sees the pre-CLI row.
    let offline = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1)
        .connect_with(options_for(&db_path).unwrap()).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM projects WHERE name='guard-probe'").fetch_one(&offline).await.unwrap();
    assert_eq!(n, 1, "row not durable after the external write session despite the guard");
}
```
Gate env: WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db wal_guard" WAI_LINT_CMD="cargo clippy -p db --all-targets -- -D warnings".


## Change
Create `crates/db/src/wal_guard.rs`. FIRST read docs/plans/wal-unlink-durability/decisions-ledger.md `## T1 mechanism evidence` and implement the mode it records (Mode::HoldRead expected; Mode::MapOnly if that is what the evidence says). In the test above, replace `ledger_mode()` with a literal of the recorded mode (e.g. `Mode::HoldRead`).

Module contents (sqlx 0.8.6, mirrors crates/db/src/lib.rs connect style):

```rust
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqliteJournalMode, SqliteSynchronous};
use sqlx::{ConnectOptions, Connection}; // ConnectOptions is REQUIRED: options.connect() is ConnectOptions::connect — without the import that call is E0599

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { MapOnly, HoldRead }

pub struct WalGuard {
    conn: SqliteConnection,
    options: SqliteConnectOptions,
    mode: Mode,
    holding_read_mark: bool,
}
```

- `pub(crate) fn options_for(db_path: &Path) -> Result<SqliteConnectOptions, sqlx::Error>` (pub(crate): tasks 030/031 reuse it — never duplicate pragma SQL): `SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.to_string_lossy()))?` then `.create_if_missing(false).journal_mode(SqliteJournalMode::Wal).synchronous(SqliteSynchronous::Normal).busy_timeout(Duration::from_secs(5))`. (create_if_missing(FALSE): the guard must never create a stray DB; the pool bootstrap already created it. Shared helper so reconnect uses identical options.)
- `pub async fn connect(db_path: &Path, mode: Mode) -> Result<Self, sqlx::Error>`: `let mut conn = options.connect().await?;` then `crate::apply_performance_pragmas(&mut conn).await?;` (crate-root private fn at lib.rs L80-107 — visible to this child module as `crate::apply_performance_pragmas`). If mode==HoldRead: `sqlx::query("BEGIN DEFERRED").execute(&mut conn).await?;` then `sqlx::query("SELECT name FROM sqlite_schema LIMIT 1").fetch_optional(&mut conn).await?;` and set holding_read_mark=true (the SELECT materialises the read-mark on the wal-index; BEGIN alone does not).
- `pub async fn is_alive(&mut self) -> bool`: `sqlx::query("SELECT 1").execute(&mut self.conn).await.is_ok()`.
- `pub async fn reconnect(&mut self) -> Result<(), sqlx::Error>`: drop the old conn, re-run connect logic from stored options+mode, restore holding_read_mark.
- HoldRead only: `pub async fn release_read_mark(&mut self)`: if holding_read_mark, `sqlx::query("COMMIT").execute(&mut self.conn).await` (log-and-continue on error — never panic in a coordination path), set false. `pub async fn reacquire_read_mark(&mut self) -> Result<(), sqlx::Error>`: BEGIN DEFERRED + the schema SELECT, set true. (The monitor calls these around its TRUNCATE checkpoint — a held read-mark blocks TRUNCATE; that is the recorded trade.)
- `pub fn guard_disabled() -> bool`: `std::env::var("VK_WAL_GUARD").map(|v| matches!(v.to_ascii_lowercase().as_str(), "off" | "0" | "false")).unwrap_or(false)` (kill-switch for the SC2 repro leg; house bool-env pattern mirrors VK_WAL_AUTO_CHECKPOINT at wal_monitor.rs L71-73).
- The wal-path helper is NOT here — it lands in wal_monitor.rs (task 020). Keep this module single-purpose.

This task introduces the new symbols `options_for()`, `connect()`, `is_alive()`, `reconnect()`, `release_read_mark()`, `reacquire_read_mark()`, and `guard_disabled()` (later tasks may call them; they are defined here).

In crates/db/src/lib.rs add exactly one line: `pub mod wal_guard;` inserted between L21 `pub mod validation;` and L22 `pub mod wal_monitor;` (alphabetical). Do NOT add a `pub use` re-export — callers use `db::wal_guard::WalGuard`.

Tests (in-module): the failing test above; plus `reconnect_restores_read_mark`: connect a HoldRead guard on a create_test_pool DB; `guard.conn.close().await.unwrap()` (in-module field access; Connection::close kills the connection); assert `!guard.is_alive().await`; `guard.reconnect().await.unwrap()`; assert `guard.is_alive().await` AND `guard.holding_read_mark` (HoldRead mode re-materialises the read-mark on reconnect); plus `guard_disabled` table test (env set to off/0/false/OFF → true; unset/anything-else → false — use serial_test::serial since it mutates env, mirroring existing dev-dep). Then run the TS3 test and confirm green.


## Allowed moves
Create crates/db/src/wal_guard.rs; add the single `pub mod wal_guard;` line in crates/db/src/lib.rs between the validation and wal_monitor decls. Nothing else. Reuse crate::apply_performance_pragmas; do not duplicate pragma SQL.


## STOP triggers
The ledger has no T1 guard-mode verdict (task 002 incomplete or inconclusive) → STOP; the mode is an evidence-gated decision, not a guess. The TS3 test shows the WAL unlinked despite HoldRead → STOP, halt code human_gate_required (spec DP2 — A5 refuted). apply_performance_pragmas is not reachable as crate::apply_performance_pragmas → STOP and report the visibility change (do not make it pub without operator sign-off).


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested (TS3). Confirm `cargo test -p db wal_guard` green and paste the test-name list into the completion report.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 010` exits 0
