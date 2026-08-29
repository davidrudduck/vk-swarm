---
id: "030"
phase: 4
title: "Salvage checkpoint on trip (run_salvage_checkpoint + succeeded/failed events)"
status: ready
depends_on: ["020"]
parallel: false
conflicts_with: ["020","021","031"]
files:
  - "crates/db/src/wal_monitor.rs"
irreversible: false
scope_test: "crates/db/src/wal_monitor.rs"
allowed_change: edit
covers_criteria: []
covers_tests: ["TS5"]
---
## Failing test (write first)
Write this test FIRST and watch it fail (run_salvage_checkpoint does not exist). IMPORTANT: read docs/plans/wal-unlink-durability/decisions-ledger.md `## T1 mechanism evidence` VERDICT 3 (A6) before writing the final assertion — if A6 was refuted, assert the salvage-FAILURE path instead of offline visibility (comment the branch).

```rust
#[tokio::test]
async fn trip_runs_salvage_checkpoint() {
    use sqlx::ConnectOptions;
    let (pool, tmp) = crate::test_utils::create_test_pool().await;
    let db_path = tmp.path().join("test.db");
    sqlx::query("INSERT INTO projects (id, name, git_repo_path) VALUES (randomblob(16), 'salvage-probe', '/tmp/salvage-probe-uniq')").execute(&pool).await.unwrap();
    let wal = wal_path_for(&db_path);
    assert!(wal.exists());
    let md = std::fs::metadata(&wal).unwrap();
    // Dedicated connection opened PRE-unlink (old shm/inode domain) — the monitor's salvage handle.
    let salvage_conn = crate::wal_guard::options_for(&db_path).unwrap().connect().await.unwrap();
    let mut mon = WalMonitor { db_path: db_path.clone(), pool: pool.clone(), metrics: crate::metrics::DbMetrics::new(), config: WalMonitorConfig::default(), last_wal_state: WalState::Present(wal_identity(&md)), tripped: false, trip_events: 0, guard: None, salvage_conn: Some(salvage_conn), last_salvage: None, /* + any fields other landed tasks added; default them */ };
    std::fs::remove_file(&wal).unwrap(); // REAL external unlink while the conns hold the inode open
    mon.check_wal_size().await;
    assert!(mon.tripped, "trip was not detected after WAL removal");
    assert!(mon.last_salvage.as_ref().is_some_and(|r| r.is_ok()), "salvage did not run through the dedicated connection: {:?}", mon.last_salvage);
    // A6 verdict TRUE branch: close EVERY original connection, then a FRESH offline connection must see the pre-trip row.
    pool.close().await;
    drop(mon);
    let offline = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1)
        .connect_with(crate::wal_guard::options_for(&db_path).unwrap()).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM projects WHERE name='salvage-probe'").fetch_one(&offline).await.unwrap();
    assert_eq!(n, 1, "salvage checkpoint did not flush pre-trip frames (A6 refuted?)");
}
```
Gate env: WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db wal_monitor" WAI_LINT_CMD="cargo clippy -p db --all-targets -- -D warnings".


## Change
Edit crates/db/src/wal_monitor.rs.

1. DEDICATED SALVAGE CONNECTION (the tournament-verified mechanism; see the spec's amended §3): WalMonitor gains `salvage_conn: Option<sqlx::sqlite::SqliteConnection>`, opened in spawn/spawn_default BEFORE the monitor task starts (pre-unlink — this connection lives in the OLD shm/inode domain, which after an external unlink is the ONLY handle guaranteed to address the orphaned WAL inode): `crate::wal_guard::options_for(&db_path)?.connect().await` (import `use sqlx::ConnectOptions;`). On connect error: log an error and continue with None (salvage then fails loudly at trip time — never panic in a coordination path). Add `last_salvage: Option<Result<(i32,i32,i32), String>>` for test observability (Ok mapped to its tuple, Err to the message).

2. `async fn run_salvage_checkpoint(&mut self) -> Result<(i32, i32, i32), sqlx::Error>` — the checkpoint runs through the DEDICATED connection, NOT the pool (a fresh pooled connection post-unlink opens the NEW empty inode and checkpoints nothing):
```rust
use sqlx::Row;
let conn = self.salvage_conn.as_mut().ok_or_else(|| {
    sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::NotConnected, "salvage connection unavailable").into())
})?;
let row = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").fetch_one(conn).await?;
Ok((row.try_get(0)?, row.try_get(1)?, row.try_get(2)?)) // busy, log frames, checkpointed frames
```

3. handle_trip ORDER (spec's amended §3): (1) emit the wal_unlinked_externally event as structured in 020; (2) run_salvage_checkpoint FIRST — writers may still be committing into the orphaned WAL and the salvage checkpoint flushes those frames too — emitting per spec §3: on Ok((busy, log_frames, checkpointed)) → `tracing::info!(event = "wal_salvage_checkpoint_succeeded", busy, log_frames, checkpointed_frames = checkpointed, "WAL salvage checkpoint succeeded")`; on Err(e) → `tracing::error!(event = "wal_salvage_checkpoint_failed", error = ?e, "WAL salvage checkpoint failed")`; store the outcome in self.last_salvage; (3) THEN task 031's refusal latch — latch-first would make the salvage checkpoint itself fail with database-locked.

4. The test's struct literal must match the landed field set (last_wal_state, tripped, trip_events, guard, salvage_conn, last_salvage — plus whatever other landed tasks added; default everything to None/false/0 except last_wal_state, which the test seeds from the real WAL metadata, and salvage_conn, which the test opens BEFORE the unlink via crate::wal_guard::options_for). The offline probe in the test uses crate::wal_guard::options_for directly (it is pub(crate) — do NOT duplicate pragma SQL).

This task introduces the new symbol `run_salvage_checkpoint()`.


## Allowed moves
Edit ONLY crates/db/src/wal_monitor.rs. Do not change run_checkpoint/run_truncate_checkpoint (scheduled paths stay as-is). Do not add the refusal latch here — that is task 031.


## STOP triggers
The ledger has no A6 verdict (task 002 incomplete) → STOP; the test's final assertion encodes an empirical verdict, not a guess. The salvage checkpoint errors consistently even in the A6-true environment → STOP and record the sqlx error; salvage may need a different connection path — do not silently skip. check_wal_size's trip path cannot reach handle_trip in the test because metadata timing differs → STOP and report rather than adding sleeps.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested (TS5). Confirm `cargo test -p db wal_monitor trip_runs_salvage_checkpoint` green and paste the assertion branch used (A6-true vs A6-refuted) with the ledger line it came from.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 030` exits 0
