---
id: "021"
phase: 3
title: "Linux inotify fast-wake for WAL removal (notify crate, poll fallback)"
status: ready
depends_on: ["020"]
parallel: false
conflicts_with: ["020","030","031"]
files:
  - "crates/db/src/wal_monitor.rs"
  - "crates/db/Cargo.toml"
  - "Cargo.lock"
siblings: ["crates/services/src/services/filesystem_watcher.rs"]
irreversible: false
scope_test: "crates/db/src/wal_monitor.rs"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
Write this test FIRST and watch it fail (function does not exist):

```rust
#[cfg(target_os = "linux")]
#[test]
fn is_wal_removal_matches_delete_and_rename_from() {
    use notify::event::{EventKind, RemoveKind, ModifyKind, RenameMode};
    let wal = std::path::PathBuf::from("/x/db.sqlite-wal");
    let other = std::path::PathBuf::from("/x/db.sqlite");
    assert!(is_wal_removal(&EventKind::Remove(RemoveKind::File), std::slice::from_ref(&wal), "db.sqlite-wal"));
    assert!(is_wal_removal(&EventKind::Modify(ModifyKind::Name(RenameMode::From)), std::slice::from_ref(&wal), "db.sqlite-wal"));
    assert!(!is_wal_removal(&EventKind::Remove(RemoveKind::File), std::slice::from_ref(&other), "db.sqlite-wal"));
    assert!(!is_wal_removal(&EventKind::Create(notify::event::CreateKind::File), std::slice::from_ref(&wal), "db.sqlite-wal"));
}
```
Gate env: WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db wal_monitor" WAI_LINT_CMD="cargo clippy -p db --all-targets -- -D warnings".


## Change
FIRST read the sibling crates/services/src/services/filesystem_watcher.rs (notify 8.2.0 + notify-debouncer-full usage) and list every exclusion/guard/structural choice it makes; justify each divergence in the decisions ledger (expected main divergence: we need RAW un-debounced events for a single known filename, so no debouncer; that is deliberate).

1. crates/db/Cargo.toml: add a target-gated dependency (notify 8.2.0 is already used by services + local-deployment — same version):
```toml
[target.'cfg(target_os = "linux")'.dependencies]
notify = "8.2.0"
```

2. crates/db/src/wal_monitor.rs, all inside `#[cfg(target_os = "linux")]` where Linux-only: add `fn is_wal_removal(kind: &notify::event::EventKind, paths: &[PathBuf], wal_basename: &str) -> bool` — true when kind is `Remove(_)` or `Modify(ModifyKind::Name(RenameMode::From))` AND any path's file_name equals wal_basename.

3. WATCH SETUP (fn `fn start_watch(db_path: &Path) -> Option<(notify::RecommendedWatcher, tokio::sync::mpsc::UnboundedReceiver<notify::Result<notify::Event>>)>`): create RecommendedWatcher with a callback forwarding into an unbounded tokio channel; `watcher.watch(db_dir, RecursiveMode::NotRecursive)`; on any error log warn and return None (degrade to poll).

4. WATCH LIFETIME — the watch is LOOP-LOCAL inside run(); do NOT add a watch field to WalMonitor (post-020 `run(mut self)` plus a select arm borrowing `&mut self.watch` while also calling `self.check_wal_size()` is a double mutable borrow — E0499): create it inside run() BEFORE the first metadata reconcile (a deletion during startup must not be missed): `let mut watch = start_watch(&self.db_path);` and compute `let wal_basename = wal_path_for(&self.db_path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();` once before the loop. Add the select arm:
```rust
event = async {
    match &mut watch {
        Some((_, rx)) => rx.recv().await,
        None => std::future::pending().await,
    }
} => {
    match event {
        Some(Ok(ev)) if is_wal_removal(&ev.kind, &ev.paths, &wal_basename) => self.check_wal_size().await,
        Some(Ok(_)) => {}
        Some(Err(e)) => tracing::warn!(error = ?e, "WAL watch error"),
        None => { watch = None; } // watcher died; re-created on the next 60s tick
    }
}
```
On each 60s tick: `if watch.is_none() { watch = start_watch(&self.db_path); if watch.is_some() { tracing::info!("WAL watch re-created"); } }`.

5. Non-Linux: the cfg gate leaves the 60s poll as the only detector — exactly the spec's documented degradation; add a debug! once at startup noting the poll-only posture.

This task introduces the new symbols `is_wal_removal()` and `start_watch()`.


## Allowed moves
Edit crates/db/src/wal_monitor.rs and crates/db/Cargo.toml only. The notify version MUST be 8.2.0 (workspace consistency). Do not add notify-debouncer-full. Do not refactor the existing select arms beyond adding the watch arm.


## STOP triggers
notify 8.2.0 API differs from what the task text assumes (EventKind/ModifyKind shape) → adapt to the real API but STOP if the watcher cannot deliver un-debounced events without the debouncer crate. Adding the dep pulls a different inotify/libc version that conflicts in Cargo.lock → STOP and report the resolution conflict. The select arm cannot be expressed without restructuring the loop → STOP and propose the restructure in the ledger before doing it.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested. Confirm `cargo test -p db wal_monitor` green and `cargo check -p db --target x86_64-unknown-linux-gnu` clean (host target).


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 021` exits 0
