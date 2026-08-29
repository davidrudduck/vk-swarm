---
id: "022"
phase: 4
title: "Wire WalGuard + WalMonitor into LocalDeployment::from_parts with shutdown ordering"
status: ready
depends_on: ["010","020","021","030","031"]
parallel: false
conflicts_with: []
files:
  - "crates/local-deployment/src/lib.rs"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — wiring task verified by the live smoke below. Gate env: WAI_TYPECHECK_CMD="cargo check -p local-deployment" WAI_TEST_CMD="cargo check -p local-deployment --tests" WAI_LINT_CMD="cargo clippy -p local-deployment --all-targets -- -D warnings".


## Change
Edit crates/local-deployment/src/lib.rs (imports at L1-38 already include `use db::DBService;` L4 and `assets::{..., database_path}` L33; the crate's Cargo.toml has sqlx 0.8.6 + tokio).

1. THREAD THE DB PATH FIRST (from_parts is a test seam — its test callers at L1340/L1373 build tempdir DBs and for_test constructs a DBService literal at L533-549; calling database_path() inside from_parts would point TESTS at the real production DB): add a `db_path: std::path::PathBuf` parameter to from_parts (add `use std::path::PathBuf;` if not already imported); the production caller (the `Self::from_parts(...)` call in new(), L660-672) passes `database_path()`; test callers pass their tempdir db path; for_test passes the path backing its DBService. The wiring block below uses this `db_path` parameter — NEVER database_path() — for both the guard and the monitor.

2. In `from_parts` (L157-505), immediately AFTER the existing block at L437-438 (`// Spawn the event compaction loop over the same live pool` / `let compaction_handle = EventCompaction::spawn(db.pool.clone(), tuning.compaction);`) and BEFORE `let deployment = Self {` (L440), insert:

```rust
// WAL durability: prevention guard + detection monitor (wal-unlink-durability).
let wal_guard = if db::wal_guard::guard_disabled() {
    tracing::warn!("WAL guard disabled via VK_WAL_GUARD; node is exposed to external WAL unlink");
    None
} else {
    match db::wal_guard::WalGuard::connect(&db_path, db::wal_guard::Mode::HoldRead).await {
        Ok(g) => {
            tracing::info!("WAL guard connected");
            Some(g)
        }
        Err(e) => {
            tracing::error!(error = ?e, "WAL guard failed to connect; continuing without prevention");
            None
        }
    }
};
let wal_monitor_handle = db::WalMonitor::spawn(
    db_path.clone(),
    db.pool.clone(),
    db.metrics.clone(),
    db::WalMonitorConfig::default(),
    wal_guard,
);
```
IMPORTANT: the `Mode::HoldRead` literal is a placeholder — FIRST read docs/plans/wal-unlink-durability/decisions-ledger.md `## T1 mechanism evidence` and use the mode task 002 recorded (HoldRead expected). The spawn call's argument order/shape must match the post-020 signature (db_path, pool, metrics, config, guard) — if 020 ordered it differently, follow the real signature.

3. Add a field to the LocalDeployment struct (it already retains compaction_handle — find that field, L44-74+): `wal_monitor_handle: db::WalMonitorHandle,` and set it in the struct literal at L467-area next to compaction_handle.

4. In `shutdown_event_services` (L851-854: currently `{ self.compaction_handle.shutdown().await; self.event_bus.shutdown().await; }`) add as the FIRST line: `self.wal_monitor_handle.shutdown().await;` — the monitor's Shutdown arm releases the guard read-mark and drops the refusal latch BEFORE the server's final TRUNCATE checkpoint (crates/server/src/main.rs L370-381) runs.

5. Keep `DBService` clone semantics untouched; the monitor takes `db.pool.clone()` and `db.metrics.clone()` exactly like EventCompaction::spawn takes db.pool.clone().


## Allowed moves
Edit ONLY crates/local-deployment/src/lib.rs: the one wiring block, one struct field, one shutdown line. Do not modify crates/server/src/main.rs (its final TRUNCATE already exists and needs no change). Do not change the Deployment trait.


## STOP triggers
The ledger has no guard-mode verdict → STOP (placeholder modes are forbidden; the mode is evidence-gated). The post-020 spawn signature does not match the wiring block and reconciling requires touching wal_monitor.rs → STOP; report the mismatch (conflict-serialisation breach — wal_monitor tasks must land first). from_parts signature drift (a caller the task did not find, or for_test cannot supply a db path) → STOP and report rather than hardcoding database_path() anywhere in from_parts.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
Boot a scratch node on :9012 with the :9012 scratch pattern (HOST=0.0.0.0 BACKEND_PORT=9012 VK_ASSET_DIR/VK_DATABASE_PATH/VK_BACKUP_DIR/VK_WORKTREE_DIR under a mktemp dir, VK_HIVE_URL/VK_NODE_API_KEY unset, exact-PID management): the log MUST contain 'WAL monitor started' and 'WAL guard connected'. Reboot with VK_WAL_GUARD=off: the log MUST contain 'WAL guard disabled via VK_WAL_GUARD'. Graceful stop both times; paste the three log lines into the decisions ledger.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 022` exits 0
