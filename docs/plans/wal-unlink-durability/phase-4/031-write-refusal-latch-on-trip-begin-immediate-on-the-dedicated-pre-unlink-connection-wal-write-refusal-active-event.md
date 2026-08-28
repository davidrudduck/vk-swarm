---
id: "031"
phase: 4
title: "Write-refusal latch on trip: BEGIN IMMEDIATE on the dedicated pre-unlink connection + wal_write_refusal_active event"
status: ready
depends_on: ["030"]
parallel: false
conflicts_with: ["020","021","030"]
files:
  - "edit crates/db/src/wal_monitor.rs"
irreversible: false
scope_test: "crates/db/src/wal_monitor.rs"
allowed_change: edit
covers_criteria: ["SC2"]
covers_tests: ["TS2"]
---
## Failing test (write first)
Write this test FIRST and watch it fail (RefusalLatch does not exist). NOTE: db::test_utils pools have NO busy_timeout, so the blocked write fails FAST — no sleeps or timeouts needed:

```rust
#[tokio::test]
async fn refusal_latch_blocks_writes_allows_reads() {
    use sqlx::ConnectOptions;
    let (pool, tmp) = crate::test_utils::create_test_pool().await;
    let db_path = tmp.path().join("test.db");
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'pre-latch', '/tmp/pre-latch-uniq')").execute(&pool).await.unwrap(); // forces the WAL into existence
    // Dedicated connection opened PRE-unlink (old shm/inode domain) — a fresh post-unlink conn would fence nobody.
    let conn = crate::wal_guard::options_for(&db_path).unwrap().connect().await.unwrap();
    std::fs::remove_file(tmp.path().join("test.db-wal")).unwrap(); // REAL external unlink
    let latch = RefusalLatch::arm(conn).await.expect("latch must arm on the old-domain connection");
    let write = sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'refusal-probe', '/tmp/refusal-probe-uniq')").execute(&pool).await;
    assert!(write.is_err(), "write succeeded despite the refusal latch");
    let read = sqlx::query("SELECT count(*) FROM projects").fetch_one(&pool).await;
    assert!(read.is_ok(), "read blocked by the refusal latch");
    drop(latch);
}
```
Gate env: WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db wal_monitor" WAI_LINT_CMD="cargo clippy -p db --all-targets -- -D warnings".


## Change
Edit crates/db/src/wal_monitor.rs.

1. The latch (post-trip posture per spec D6 AS AMENDED at the decompose tournament: REFUSE WRITES, STAY UP — enforced by a held write transaction on the monitor's DEDICATED pre-unlink connection, which shares the OLD shm domain with the orphaned writers):
```rust
struct RefusalLatch { _conn: sqlx::sqlite::SqliteConnection }
impl RefusalLatch {
    async fn arm(mut conn: sqlx::sqlite::SqliteConnection) -> Result<Self, sqlx::Error> {
        sqlx::query("BEGIN IMMEDIATE").execute(&mut conn).await?;
        Ok(Self { _conn: conn })
    }
}
```
MECHANISM (tournament-verified — do not 'simplify'): WAL-mode writers coordinate through the shm segment. A FRESH post-unlink connection coordinates via a NEW shm inode, so its BEGIN EXCLUSIVE fences NOBODY (same-process fcntl locks do not self-conflict; the node's pooled connections still coordinate via the deleted shm). The monitor's salvage_conn was opened BEFORE the unlink, so its BEGIN IMMEDIATE takes the RESERVED lock in the OLD domain: old-domain writers then fail fast with database-locked while readers (which need no RESERVED lock) continue. The latch NEVER commits or rolls back; it is dropped only in the Shutdown arm (020's drop order: release guard read-mark, drop refusal latch, then ack).

2. WalMonitor gains `refusal: Option<RefusalLatch>`. Extend handle_trip AFTER 030's salvage step (salvage FIRST — a write committing during salvage is flushed by that same checkpoint; latch-first would block salvage): `match self.salvage_conn.take()` — on Some(conn): `match RefusalLatch::arm(conn).await { Ok(l) => { self.refusal = Some(l); emit ONCE `tracing::error!(event = "wal_write_refusal_active", path = %self.db_path.display(), remediation = "writes are refused until the node is restarted", "WAL write refusal active");` } Err(e) => fail-closed below }`; on None or on arm Err(e): FAIL CLOSED (spec's amended §3): `tracing::error!(event = "wal_write_refusal_active", armed = false, error = ?e, "write-refusal latch could not be armed; closing the pool (D6 deviation: refuse-everything)")` then `self.pool.close().await;` — with no old-domain connection the latch cannot fence, so D6's refuse-writes-stay-up degrades to refuse-EVERYTHING (writes AND reads fail fast). The deviation is logged loudly, never silent.

3. This task introduces the new symbol `arm()`.


## Allowed moves
Edit ONLY crates/db/src/wal_monitor.rs. Do not surface the refusal through the API layer (out of scope — the pooled write error IS the loud failure the spec contracts). Do not retry-loop the arm; one attempt + event.


## STOP triggers
The latch cannot arm in the test because the dedicated connection did not survive the unlink (arm returns Err) → STOP; without an old-domain connection the latch cannot fence — this is a DP-level D6 re-settle, escalate (do NOT silently swap in a fresh connection: a fresh conn coordinates via the NEW shm and fences nobody). The pooled write SUCCEEDS despite the held BEGIN IMMEDIATE on the old-domain conn → STOP; the tournament-verified shm-domain mechanism is wrong on this host and the enforcement needs re-investigation. The latch also blocks pool READS → STOP; record the observation — D6's reads-continue property is violated and the mechanism needs operator re-decision.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested (TS2). Confirm `cargo test -p db wal_monitor refusal_latch` green.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 031` exits 0
