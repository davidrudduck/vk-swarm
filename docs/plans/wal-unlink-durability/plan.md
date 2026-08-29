# wal-unlink-durability Plan

## Spec
docs/superpowers/specs/2026-08-28-wal-unlink-durability.md

## Approach
Layered fix per the frozen spec: a prevention guard (WalGuard — a dedicated connection holding the WAL wal-index so an external sqlite3 CLI close cannot unlink the live WAL) plus a detection/refusal net (the currently-dead WalMonitor revived, extended with an inode-based external-unlink watch, a salvage checkpoint, and a write-refusal latch). Phase 1 builds the two-leg live repro harness (001 — mechanics only) and then pins the unlink mechanism empirically (002): the incident's lock-state trigger window must be known before the harness can go red on current code (on a fresh scratch node the pool holds the wal-index and the external close does NOT unlink — operator-approved re-sequence 2026-08-29 moved the red proof + SC4/TS4 coverage from 001 to 002; operator-approved re-plan 2026-08-30 settled the vector: the trip stimulus is an external WRITE session — confirmed reproducible — after the read-only flow proved non-reproducible under every probed condition). The guard's operating mode is an evidence-gated decision (spec D4), so task 002 records the MapOnly-vs-HoldRead verdict and the A6 salvage verdict in the decisions ledger, and later tasks READ that ledger rather than guessing.

Phases 2-4 then implement bottom-up inside crates/db: WalGuard as a new module (Phase 2), the monitor's inode-transition classification + wal-path derivation fix + guard ownership (Phase 3), then the trip response — salvage checkpoint, write-refusal latch — and the single wiring seam in LocalDeployment::from_parts (Phase 4). All db-layer logic lands as pure/unit-testable functions first (transition classification, wal_path_for, is_wal_removal) with the live-CLI behaviour proved by TS3 and by the repro script. Phase 5 is the ship gate: run the frozen verify_cmd to green on both legs and record the SC3 no-regression evidence (journal_mode=wal + write-latency timings vs a main-built baseline binary).

Traps pre-empted from prior ledgers and recon: (1) PathBuf::with_extension("sqlite-wal") yields test.sqlite-wal for a test.db file but the real WAL is test.db-wal — derivation is fixed via format!("{}-wal") in one helper; (2) db::test_utils pools have NO busy_timeout, so lock-conflict writes fail fast — TS2 relies on this; (3) all /api routes except /api/health require a browser session cookie, so the repro script seeds browser_sessions via SQL with the node DOWN (two-boot pattern) using SHA-256 hex token hashes and 16-byte X'<hex>' UUID blobs; (4) process management is exact-PID only (never pkill) and port 9012 must be preflight-checked; (5) tests must use db::test_utils (never hand-rolled CREATE TABLE) and gate on the sqlite3 CLI with the skip_without_db! eprintln-and-return idiom.


## Phases
- **Phase 1: evidence-and-repro** — Build the two-leg live repro script (red on current code) and record the empirical verdicts it unlocks: unlinker identity, guard mode (MapOnly vs HoldRead), A6 salvage viability.
- **Phase 2: wal-guard** — WalGuard module in crates/db: dedicated connection holding the WAL wal-index in the ledger-recorded mode, with reconnect/read-mark API and the VK_WAL_GUARD kill-switch.
- **Phase 3: detection** — Revive WalMonitor: correct WAL path derivation, inode-based external-unlink classification, the wal_unlinked_externally event, guard ownership/health, and the Linux inotify fast-wake.
- **Phase 4: trip-response-and-wiring** — Salvage checkpoint + write-refusal latch on trip; wire guard+monitor into LocalDeployment::from_parts with graceful shutdown ordering.
- **Phase 5: ship-gate** — Run the frozen verify_cmd green on both legs; record SC3 evidence (journal_mode=wal, latency timings vs main baseline) in the decisions ledger.

## Tasks

| id | phase | title | deps | conflicts |
|---|---:|---|---|---|
| 001 | 1 | Live repro script: two-leg WAL-unlink harness (harness mechanics; red proved at 002) | dep: none | conflicts: 002 |
| 002 | 1 | T1 evidence: pin unlink mechanism, pick guard mode, verdict A6 salvage | dep: 001 | conflicts: 001 040 |
| 010 | 2 | WalGuard module: dedicated wal-index-holding guard connection + VK_WAL_GUARD kill-switch | dep: 002 | conflicts: none |
| 020 | 3 | WalMonitor revival: wal-path fix, inode transition classification, wal_unlinked_externally event, guard ownership | dep: 010 | conflicts: 021 030 031 |
| 021 | 3 | Linux inotify fast-wake for WAL removal (notify crate, poll fallback) | dep: 020 | conflicts: 020 030 031 |
| 030 | 4 | Salvage checkpoint on trip (run_salvage_checkpoint + succeeded/failed events) | dep: 020 | conflicts: 020 021 031 |
| 031 | 4 | Write-refusal latch on trip: BEGIN IMMEDIATE on the dedicated pre-unlink connection + wal_write_refusal_active event | dep: 030 | conflicts: 020 021 030 |
| 022 | 4 | Wire WalGuard + WalMonitor into LocalDeployment::from_parts with shutdown ordering | dep: 010 020 021 030 031 | conflicts: none |
| 040 | 5 | Ship gate: verify_cmd green on both legs + SC3 no-regression evidence | dep: 001 022 | conflicts: 002 |
