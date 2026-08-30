---
id: "020"
phase: 3
title: "WalMonitor revival: wal-path fix, inode transition classification, wal_unlinked_externally event, guard ownership"
status: ready
depends_on: ["010"]
parallel: false
conflicts_with: ["021","030","031"]
files:
  - "crates/db/src/wal_monitor.rs"
irreversible: false
scope_test: "crates/db/src/wal_monitor.rs"
allowed_change: edit
covers_criteria: []
covers_tests: ["TS1"]
---
## Failing test (write first)
Write these tests FIRST and watch them fail (functions/types do not exist yet):

```rust
#[test]
fn wal_path_for_appends_dash_wal() {
    assert_eq!(wal_path_for(Path::new("/tmp/test.db")), PathBuf::from("/tmp/test.db-wal"));
    assert_eq!(wal_path_for(Path::new("/data/db.sqlite")), PathBuf::from("/data/db.sqlite-wal"));
}

#[test]
fn wal_transition_classifies_all_cases() {
    assert_eq!(wal_transition(WalState::Absent, WalState::Absent), WalTransition::Unchanged);
    assert_eq!(wal_transition(WalState::Absent, WalState::Present(None)), WalTransition::Appeared);
    assert_eq!(wal_transition(WalState::Present(None), WalState::Absent), WalTransition::Vanished);
    assert_eq!(wal_transition(WalState::Present(Some(1)), WalState::Present(Some(2))), WalTransition::Replaced);
    assert_eq!(wal_transition(WalState::Present(Some(2)), WalState::Present(Some(2))), WalTransition::Unchanged);
    // Identity unknown on both sides (non-unix): a replace cannot be proven → Unchanged.
    assert_eq!(wal_transition(WalState::Present(None), WalState::Present(None)), WalTransition::Unchanged);
}
```

Plus MONITOR-LEVEL trips via crate::test_utils::create_test_pool() (construct the WalMonitor struct in-module — the field set grows in later tasks; default every detector field to None/false/0 and seed last_wal_state from the real WAL metadata like task 030's test shows; seed `wal_ever_present` true wherever last_wal_state is seeded Present):
(a) `vanished_trips`: insert a projects row (forces the WAL into existence), `std::fs::remove_file(wal_path_for(&db_path))`, `mon.check_wal_size().await` → assert `mon.tripped` and `mon.trip_events == 1`.
(b) `no_wal_yet_does_not_trip`: fresh test pool with NO writes yet (no WAL file; boot state Absent) → check_wal_size → assert `!mon.tripped` (NoWalYet is benign).
(c) `trip_is_idempotent`: after (a), remove any re-created WAL again and call check_wal_size twice more → assert `mon.trip_events == 1` still (the early-return keeps tracking last_wal_state without re-firing).
Gate env: WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db wal_monitor" WAI_LINT_CMD="cargo clippy -p db --all-targets -- -D warnings".


## Change
Edit crates/db/src/wal_monitor.rs (read the whole file first; current shape: config L40-73, handle L91-127, struct L129, spawn L140/spawn_default L160, run loop L168, check_wal_size L225, run_checkpoint L278, run_truncate_checkpoint L319, get_wal_size L367, tests L395-453).

1. WAL PATH FIX (trap: `with_extension("sqlite-wal")` turns test.db into test.sqlite-wal but the real WAL is test.db-wal): add `fn wal_path_for(db_path: &Path) -> PathBuf { PathBuf::from(format!("{}-wal", db_path.display())) }` and use it at BOTH existing call sites — check_wal_size L226 (replace `self.db_path.with_extension("sqlite-wal")`) and get_wal_size L368-369 (same replacement).

2. WAL IDENTITY + TRANSITION (cross-platform — Windows targets exist in .github/workflows/pre-release.yml L182-193, so an unconditional `use std::os::unix::fs::MetadataExt` breaks that build):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalState { Absent, Present(Option<u64>) }

#[cfg(unix)]
fn wal_identity(md: &std::fs::Metadata) -> Option<u64> { use std::os::unix::fs::MetadataExt; Some(md.ino()) }
#[cfg(not(unix))]
fn wal_identity(_md: &std::fs::Metadata) -> Option<u64> { None }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalTransition { Unchanged, Appeared, Vanished, Replaced }

fn wal_transition(last: WalState, current: WalState) -> WalTransition {
    match (last, current) {
        (WalState::Absent, WalState::Absent) => WalTransition::Unchanged,
        (WalState::Absent, WalState::Present(_)) => WalTransition::Appeared,
        (WalState::Present(_), WalState::Absent) => WalTransition::Vanished,
        (WalState::Present(Some(a)), WalState::Present(Some(b))) if a != b => WalTransition::Replaced,
        (WalState::Present(_), WalState::Present(_)) => WalTransition::Unchanged,
    }
}
```
(Replaced is classified ONLY when both identities exist; on non-unix, inode identity is unavailable so Present→Present is Unchanged — Appeared/Vanished still classify everywhere. This is the spec's documented non-Linux degradation.)

3. EVENT STRUCT (frozen spec §3 shape — the spec's literal field set wins over house message-last style):
```rust
struct UnlinkedEvent { event: &'static str, path: PathBuf, wal_path: PathBuf, last_inode: Option<u64>, remediation: &'static str }
```
(integrated-review amendment 2026-08-30: `path` carries the DB path, not the WAL path — SC2 and the live harness both assert that the emitted line names the database file. The WAL path travels in the separate `wal_path` field.)

4. STRUCT + SPAWN: add fields to WalMonitor: `last_wal_state: WalState`, `wal_ever_present: bool`, `tripped: bool`, `trip_events: u32` (test-observable idempotence counter), `guard: Option<crate::wal_guard::WalGuard>`. The monitor now owns mutable state across awaits: change `async fn run(self)` → `run(mut self)`, `check_wal_size(&self)` → `&mut self`, and add `async fn handle_trip(&mut self)`. Change BOTH spawn and spawn_default to take an extra trailing param `guard: Option<crate::wal_guard::WalGuard>` (both are dead code today — no callers to update; task 022 adds the first). SEED the initial state synchronously in spawn BEFORE spawning the task: `let last_wal_state = match std::fs::metadata(wal_path_for(&db_path)) { Ok(md) => WalState::Present(wal_identity(&md)), Err(_) => WalState::Absent };` and `let wal_ever_present = matches!(last_wal_state, WalState::Present(_));` — the first check must compare against boot-time state (a pre-existing WAL is not Appeared).

5. check_wal_size REWORK: metadata via `std::fs::metadata(&wal_path)`; Ok(md) → WalState::Present(wal_identity(&md)); Err(NotFound) → WalState::Absent (REPLACE the NotFound→0 swallow at L230-233); other Err → warn and keep prior state. FIRST: `if self.tripped { self.last_wal_state = current; return; }` — after the first trip the monitor keeps tracking state but NEVER re-fires the event/salvage/latch (kills the 60s re-fire loop and any latch re-arm contention). Otherwise compute `wal_transition(self.last_wal_state, current)`, then classify as follows and ALWAYS store `self.last_wal_state = current` afterwards.

integrated-review amendment 2026-08-30 (WalState seeding / leg-B race): a WAL that the monitor has SEEN cannot legitimately go missing. Seeding alone is not enough — in the live leg-B sequence spawn seeds Absent (the node has not written yet), the first API write creates the WAL, and the external removal can land before the next tick, so a naive `last == Absent` reading classifies the incident as benign. Therefore:
   - Appeared (Absent→Present): `self.wal_ever_present = true;` UPDATE last_wal_state and DO NOT trip — a WAL coming into existence is normal. Continue into the existing size-threshold logic.
   - Unchanged/Replaced while Present: `self.wal_ever_present = true;` Unchanged continues the existing size-threshold logic unchanged (size from the metadata when Present); Replaced is an external unlink (see below).
   - Present→Vanished, OR Replaced, OR any observation of `WalState::Absent` while `self.wal_ever_present` is true → external unlink.
   - Absent while `self.wal_ever_present` is false → benign NoWalYet (the WAL was never observed present this boot: debug! only, NO trip). This benign reading is available ONLY in that never-observed case.

On the external-unlink classification: `self.tripped = true; self.trip_events += 1;`, build UnlinkedEvent{event:"wal_unlinked_externally", path: self.db_path.clone(), wal_path: wal_path.clone(), last_inode: (the identity of the last Present state, if known), remediation:"node will refuse writes; restart the node after investigating"} and emit `tracing::warn!(event = "wal_unlinked_externally", path = %event.path.display(), wal_path = %event.wal_path.display(), last_inode = ?event.last_inode, remediation = event.remediation, "WAL unlinked externally")` — `path` is the DB path per SC2 — then `self.handle_trip().await` (in THIS task handle_trip only emits the event + sets tripped; tasks 030/031 extend it with salvage and the refusal latch).

6. GUARD HEALTH + TRUNCATE COORDINATION in the run loop: on each 60s tick, if let Some(guard) = &mut self.guard: `if !guard.is_alive().await { match guard.reconnect().await { Ok(_) => tracing::warn!(event = "wal_guard_reconnected", "WAL guard reconnected"), Err(e) => { tracing::error!(error = ?e, "WAL guard reconnect failed"); if !self.tripped { self.tripped = true; self.trip_events += 1; tracing::error!(event = "wal_guard_unavailable", "WAL guard unavailable; treating as durability trip"); self.handle_trip().await; } } } }` (prevention is GONE when the guard cannot be restored — escalate to the trip path ONCE; do not keep retrying silently). Around the truncate tick AND the Shutdown arm: `if let Some(g) = &mut self.guard { g.release_read_mark().await; }` before run_truncate_checkpoint, and `if let Err(e) = g.reacquire_read_mark().await { tracing::error!(...) }` after (skip reacquire in the Shutdown arm — the node is stopping). A held read-mark blocks TRUNCATE; this window is the recorded trade under HoldRead.

integrated-review amendment 2026-08-30: both calls are HOLDREAD-ONLY and MUST be no-ops in the selected MapOnly mode — task 010 gates them on the guard's own mode, and this task must not re-acquire unconditionally. Without the gate the first TRUNCATE tick would upgrade a MapOnly guard into a read-mark-holding one and then permanently block every subsequent TRUNCATE, including the node's shutdown checkpoint.

7. ACKED SHUTDOWN (the current handle is fire-and-forget: `let _ = send` then the task is detached, wal_monitor.rs L122-125/L153-156 — the node's final TRUNCATE at server main.rs L370-381 can race the monitor's held read-mark): change the command variant to `Shutdown(tokio::sync::oneshot::Sender<()>)`. The Shutdown arm, IN THIS ORDER (integrated-review amendment 2026-08-30: the node's final TRUNCATE checkpoint must not race a live refusal latch, so the latch has to be gone before the ack is sent): (i) release the guard read-mark, HoldRead only, and skip reacquire; (ii) drop the refusal latch if task 031's `refusal` field is Some (`self.refusal = None;` — dropping the held write transaction's connection releases the RESERVED lock); (iii) run the existing shutdown path; (iv) `let _ = ack.send(());` before returning. WalMonitorHandle::shutdown: create a oneshot, send Shutdown(ack_tx) (a closed channel means already-stopped — fine), then `match tokio::time::timeout(std::time::Duration::from_secs(10), ack_rx).await { Ok(_) => {} Err(_) => tracing::error!("WAL monitor did not ack shutdown within 10s") }` — NEVER report success on timeout.

8. Update existing tests that break (test_get_wal_size_nonexistent etc. still pass; the wal path change makes get_wal_size honour test.db-wal naming).

This task introduces the new symbols `wal_path_for()`, `wal_identity()`, `wal_transition()`, and `handle_trip()`.


## Allowed moves
Edit ONLY crates/db/src/wal_monitor.rs. spawn/spawn_default have zero callers — the signature change is safe; do not add call sites (task 022 owns wiring). Do not touch crates/db/Cargo.toml (task 021 owns the notify dep).


## STOP triggers
Existing wal_monitor tests cannot be made green without deleting assertions → STOP; report the conflict. The NotFound→0 swallow at L230-233 is load-bearing for some other caller (grep shows one) → STOP and report instead of reworking. The struct cannot own WalGuard without a circular dependency (wal_guard.rs referencing wal_monitor) → STOP; 010 defined no such reference, so report what changed.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
N/A — unit-tested (TS1). Confirm `cargo test -p db wal_monitor` green including the two new tests, and that the file compiles on non-Linux cfg (wal_transition uses MetadataExt::ino — gate the import with cfg(unix) if clippy warns on other targets; the dev host is Linux).


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 020` exits 0
