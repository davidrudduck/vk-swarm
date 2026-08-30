---
id: "031"
phase: 4
title: "Write-refusal latch on trip: BEGIN IMMEDIATE on the dedicated pre-unlink connection + wal_write_refusal_active event"
status: ready
depends_on: ["030"]
parallel: false
conflicts_with: ["020","021","030"]
files:
  - "crates/db/src/wal_monitor.rs"
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
    // Hold one pool connection across unlink. sqlx retires a connection after a
    // locked write; the next `&pool` acquire is a FRESH post-unlink conn that
    // fails with SQLITE_IOERR (code 522), which is NOT a latch block. D6's
    // reads-continue is about OLD-domain pooled connections (execute-time
    // amendment 2026-08-30).
    let mut pooled = pool.acquire().await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'pre-latch', '/tmp/pre-latch-uniq')").execute(&mut *pooled).await.unwrap(); // forces the WAL into existence
    // Dedicated connection opened PRE-unlink (old shm/inode domain) — a fresh post-unlink conn would fence nobody.
    let mut conn = crate::wal_guard::options_for(&db_path).unwrap().connect().await.unwrap();
    // Dummy read maps the wal-index — connect alone does not put the connection in the old shm domain.
    sqlx::query("SELECT count(*) FROM sqlite_master").fetch_one(&mut conn).await.unwrap();
    // integrated-review amendment 2026-08-30: fault injection removes BOTH files, matching the live harness.
    std::fs::remove_file(tmp.path().join("test.db-wal")).unwrap(); // REAL external unlink
    let _ = std::fs::remove_file(tmp.path().join("test.db-shm"));
    let latch = RefusalLatch::arm(conn).await.expect("latch must arm on the old-domain connection");
    let write = sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'refusal-probe', '/tmp/refusal-probe-uniq')").execute(&mut *pooled).await;
    let write_code = write.as_ref().err()
        .and_then(|e| e.as_database_error())
        .and_then(|e| e.code())
        .map(|c| c.into_owned());
    assert_eq!(write_code.as_deref(), Some("5"),
        "write must be refused by the latch with SQLITE_BUSY (code 5), got {write:?}");
    let read: Result<i64, sqlx::Error> = sqlx::query_scalar("SELECT count(*) FROM projects").fetch_one(&mut *pooled).await;
    assert!(read.is_ok(), "read blocked on the held old-domain connection: {read:?}");
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
MECHANISM (tournament-verified — do not 'simplify'): WAL-mode writers coordinate through the shm segment. A FRESH post-unlink connection coordinates via a NEW shm inode, so its BEGIN EXCLUSIVE fences NOBODY (same-process fcntl locks do not self-conflict; the node's pooled connections still coordinate via the deleted shm). The monitor's salvage_conn was opened BEFORE the unlink, so its BEGIN IMMEDIATE takes the RESERVED lock in the OLD domain: old-domain writers then fail fast with database-locked while readers (which need no RESERVED lock) continue. The latch NEVER commits or rolls back; it is dropped only in the Shutdown arm. integrated-review amendment 2026-08-30 — this is an explicit contract with task 020's Shutdown arm, in this order: release the guard read-mark (HoldRead only), DROP THE REFUSAL LATCH (`self.refusal = None;`), run the shutdown path, and only THEN send the ack. The ack is what releases the node's final TRUNCATE checkpoint, and a still-live latch would block it.

2. WalMonitor gains `refusal: Option<RefusalLatch>`. Extend handle_trip AFTER 030's salvage step (salvage FIRST — a write committing during salvage is flushed by that same checkpoint; latch-first would block salvage): `match self.salvage_conn.take()` — on Some(conn): `match RefusalLatch::arm(conn).await { Ok(l) => { self.refusal = Some(l); emit ONCE `tracing::error!(event = "wal_write_refusal_active", path = %self.db_path.display(), remediation = "writes are refused until the node is restarted", "WAL write refusal active");` (integrated-review amendment 2026-08-30: the `path` field is the DB path, exactly as in task 020's unlink event — SC2 and the live harness both read the DB path off these lines) } Err(e) => fail-closed below }`; on None or on arm Err(e): FAIL CLOSED (spec's amended §3): `tracing::error!(event = "wal_write_refusal_active", armed = false, error = ?e, "write-refusal latch could not be armed; closing the pool (D6 deviation: refuse-everything)")` then `self.pool.close().await;` — with no old-domain connection the latch cannot fence, so D6's refuse-writes-stay-up degrades to refuse-EVERYTHING (writes AND reads fail fast). The deviation is logged loudly, never silent.

3. This task introduces the new symbol `arm()`.


## Allowed moves
Edit ONLY crates/db/src/wal_monitor.rs. Do not surface the refusal through the API layer (out of scope — the pooled write error IS the loud failure the spec contracts). Do not retry-loop the arm; one attempt + event.


## STOP triggers
The latch cannot arm in the test because the dedicated connection did not survive the unlink (arm returns Err) → STOP; without an old-domain connection the latch cannot fence — this is a DP-level D6 re-settle, escalate (do NOT silently swap in a fresh connection: a fresh conn coordinates via the NEW shm and fences nobody). The pooled write SUCCEEDS despite the held BEGIN IMMEDIATE on the old-domain conn → STOP; the tournament-verified shm-domain mechanism is wrong on this host and the enforcement needs re-investigation. The latch also blocks pool READS on the held old-domain pooled connection (SQLITE_BUSY / code 5) → STOP; record the observation — D6's reads-continue property is violated and the mechanism needs operator re-decision. A SQLITE_IOERR (code 522) on a fresh `&pool` acquire after unlink is NOT this trigger (sqlx retired the old conn; that is a new shm domain). The read on the **held** old-domain pooled connection fails with SQLITE_IOERR (code 522) rather than SQLITE_BUSY → STOP; that is neither the fresh-acquire exemption nor a latch block — the held connection lost its old-domain shm mapping and the test's fault-injection premise is wrong. Record the observed code and escalate.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested (TS2). Confirm `cargo test -p db wal_monitor refusal_latch` green.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 031` exits 0
