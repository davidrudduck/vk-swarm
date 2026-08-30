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
    // (integrated-review amendment 2026-08-30: connect alone does not map the wal-index — the dummy read does.)
    let mut salvage_conn = crate::wal_guard::options_for(&db_path).unwrap().connect().await.unwrap();
    sqlx::query("SELECT count(*) FROM sqlite_master").fetch_one(&mut salvage_conn).await.unwrap();
    let mut mon = WalMonitor { db_path: db_path.clone(), pool: pool.clone(), metrics: crate::metrics::DbMetrics::new(), config: WalMonitorConfig::default(), last_wal_state: WalState::Present(wal_identity(&md)), tripped: false, trip_events: 0, guard: None, salvage_conn: Some(salvage_conn), last_salvage: None, /* + any fields other landed tasks added; default them */ };
    // integrated-review amendment 2026-08-30: fault injection must match the live harness — remove BOTH -wal and -shm.
    std::fs::remove_file(&wal).unwrap(); // REAL external unlink while the conns hold the inode open
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
    // A6 pre-stop differential (ledger L411-421). BOTH sides required: the real salvage
    // checkpoint on an unlinked WAL returns (0,0,0), identical to a stub Ok((0,0,0)) —
    // only the main-file content discriminates. Each read uses a FRESH immutable
    // connection (immutable=1 tells SQLite the file never changes, so pages are cached).
    async fn main_file_probe(db_path: &std::path::Path) -> i64 {
        let p = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1)
            .connect_with(crate::wal_guard::options_for(db_path).unwrap().immutable(true))
            .await.unwrap();
        // schema itself may live only in the WAL — do not let `no such table` panic
        let has: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='projects'")
            .fetch_one(&p).await.unwrap();
        let n: i64 = if has == 0 { 0 } else {
            sqlx::query_scalar("SELECT count(*) FROM projects WHERE name='salvage-probe'")
                .fetch_one(&p).await.unwrap()
        };
        p.close().await;
        n
    }
    let before = main_file_probe(&db_path).await;
    assert_eq!(before, 0, "pre-trip main file already holds the row — no differential to measure (an earlier checkpoint flushed it); the A6 assertion would be hollow");
    mon.check_wal_size().await;
    assert!(mon.tripped, "trip was not detected after WAL removal");
    assert!(mon.last_salvage.as_ref().is_some_and(|r| r.is_ok()), "salvage did not run through the dedicated connection: {:?}", mon.last_salvage);
    // busy must be 0 — an Ok row with busy=1 is a BLOCKED checkpoint, not a salvage (ledger VERDICT 2 recorded [(1,880,19)]).
    assert!(matches!(mon.last_salvage.as_ref(), Some(Ok((0, _, _)))), "salvage checkpoint reported busy: {:?}", mon.last_salvage);
    // pool+mon+salvage_conn are STILL OPEN — no shutdown checkpoint has run.
    let after = main_file_probe(&db_path).await;
    assert_eq!(after, 1, "salvage checkpoint did not flush pre-trip frames into the main file (A6 pre-stop differential; before={before})");
    pool.close().await;
    drop(mon);
}
```
Gate env: WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db wal_monitor" WAI_LINT_CMD="cargo clippy -p db --all-targets -- -D warnings".


## Change
Edit crates/db/src/wal_monitor.rs.

1. DEDICATED SALVAGE CONNECTION (the tournament-verified mechanism; see the spec's amended §3): WalMonitor gains `salvage_conn: Option<sqlx::sqlite::SqliteConnection>`, holding a connection opened BEFORE the monitor task starts (pre-unlink — this connection lives in the OLD shm/inode domain, which after an external unlink is the ONLY handle guaranteed to address the orphaned WAL inode).

   OPENED BY THE CALLER, NOT BY spawn (integrated-review amendment 2026-08-30: spawn/spawn_default at wal_monitor.rs L140-166 are SYNCHRONOUS and task 022 calls them without `.await`, so an `.await` inside them would not compile): the CALLER — the async DBService init / `from_parts` wiring in task 022 — opens the connection and passes the resulting `Option<SqliteConnection>` INTO spawn as a trailing parameter, e.g. `WalMonitor::spawn_default(config, db_path, salvage_conn)` / `WalMonitor::spawn(db_path, pool, metrics, config, guard, salvage_conn)`. spawn stores it on the struct and stays synchronous.

   The public opener already exists: task 010 defines `crate::wal_guard::open_salvage_connection` (connect + pragmas + the dummy read below), because `options_for` is pub(crate) and the cross-crate task-022 caller needs a public entry point. Do NOT define a second opener here.

   DUMMY READ AT OPEN (integrated-review amendment 2026-08-30, evidence B2/B3): after connecting, run `sqlx::query("SELECT count(*) FROM sqlite_master").fetch_one(&mut conn).await?`. Connect-only does NOT map the wal-index/shm segment — the ledger's VERDICT-2 and VERDICT-3 probes each issued a SELECT before their lock/checkpoint observation, and an unmapped connection is not in the old shm domain at all. Apply the same rule to any other dedicated old-domain connection.

   On connect or dummy-read error: log an error and pass None (salvage then fails loudly at trip time — never panic in a coordination path). Add `last_salvage: Option<Result<(i32,i32,i32), String>>` for test observability (Ok mapped to its tuple, Err to the message).

2. `async fn run_salvage_checkpoint(&mut self) -> Result<(i32, i32, i32), sqlx::Error>` — the checkpoint runs through the DEDICATED connection, NOT the pool (a fresh pooled connection post-unlink opens the NEW empty inode and checkpoints nothing):
```rust
use sqlx::Row;
let conn = self.salvage_conn.as_mut().ok_or_else(|| {
    sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::NotConnected, "salvage connection unavailable").into())
})?;
let row = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").fetch_one(conn).await?;
let (busy, log_frames, checkpointed): (i32, i32, i32) = (row.try_get(0)?, row.try_get(1)?, row.try_get(2)?);
// integrated-review amendment 2026-08-30: an Ok row with a non-zero first column is a BLOCKED
// checkpoint, not a salvage — the ledger's VERDICT 2 recorded [(1, 880, 19)] for exactly that
// case. Only a zero first column counts as success; anything else is mapped to an error here so
// the caller emits the failed event.
if busy != 0 {
    return Err(sqlx::Error::Protocol(format!(
        "salvage checkpoint blocked (busy={busy}, log_frames={log_frames}, checkpointed={checkpointed})"
    )));
}
Ok((busy, log_frames, checkpointed)) // busy (always 0 here), log frames, checkpointed frames
```

3. handle_trip ORDER (spec's amended §3): (1) emit the wal_unlinked_externally event as structured in 020; (2) run_salvage_checkpoint FIRST — writers may still be committing into the orphaned WAL and the salvage checkpoint flushes those frames too — emitting per spec §3: on Ok((busy, log_frames, checkpointed)) — which by the guard above means busy == 0 — → `tracing::info!(event = "wal_salvage_checkpoint_succeeded", busy, log_frames, checkpointed_frames = checkpointed, "WAL salvage checkpoint succeeded")`; on Err(e), INCLUDING the blocked-checkpoint case mapped above → `tracing::error!(event = "wal_salvage_checkpoint_failed", error = ?e, "WAL salvage checkpoint failed")` (integrated-review amendment 2026-08-30: a checkpoint that returns Ok with a non-zero busy column is a failure, never a success); store the outcome in self.last_salvage; (3) THEN task 031's refusal latch — latch-first would make the salvage checkpoint itself fail with database-locked.

4. The test's struct literal must match the landed field set (last_wal_state, tripped, trip_events, guard, salvage_conn, last_salvage — plus whatever other landed tasks added; default everything to None/false/0 except last_wal_state, which the test seeds from the real WAL metadata, and salvage_conn, which the test opens BEFORE the unlink via crate::wal_guard::options_for AND primes with the dummy read). The offline probe in the test uses crate::wal_guard::options_for directly (it is pub(crate) — do NOT duplicate pragma SQL). Its fault injection removes BOTH the `-wal` and the `-shm` file, matching the live harness's injection step (integrated-review amendment 2026-08-30: harness parity — a wal-only removal is not the incident post-state).

This task introduces the new symbol `run_salvage_checkpoint()`.


## Allowed moves
Edit ONLY crates/db/src/wal_monitor.rs. Do not change run_checkpoint/run_truncate_checkpoint (scheduled paths stay as-is). Do not add the refusal latch here — that is task 031.


## STOP triggers
The ledger has no A6 verdict (task 002 incomplete) → STOP; the test's final assertion encodes an empirical verdict, not a guess. The salvage checkpoint errors consistently even in the A6-true environment → STOP and record the sqlx error; salvage may need a different connection path — do not silently skip. check_wal_size's trip path cannot reach handle_trip in the test because metadata timing differs → STOP and report rather than adding sleeps.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested (TS5). Confirm `cargo test -p db wal_monitor trip_runs_salvage_checkpoint` green and paste the assertion branch used (A6-true pre-stop immutable n==1 vs A6-refuted) with the ledger line it came from.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 030` exits 0
