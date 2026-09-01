---
doc_type: release-notes
date: 2026-08-31
workstream: wal-unlink-durability
pr: 479
merge_commit: 72f20ea031072f0a2edfba5dd484e243cf27c170
---

# Release notes — wal-unlink-durability (2026-08-31)

Fixes backlog **F-2026-08-28-01** (medium): an external sqlite3 CLI session on
the live node DB unlinked the node's WAL/SHM on clean close; subsequent node
writes committed into the unlinked inode and were silently lost on node exit
(observed 2026-08-28: a deleted task resurrected after a graceful stop).

## What shipped

- **Prevention — `WalGuard`** (`crates/db/src/wal_guard.rs`): dedicated
  connection holding the WAL wal-index mapped, so an external close can never
  become the last locker able to checkpoint+unlink. Kill-switch
  `VK_WAL_GUARD=off`.
- **Detection — revived `WalMonitor`** (`crates/db/src/wal_monitor.rs`):
  inotify fast-wake + 60s poll fallback, inode-transition classification, named
  WARN `wal_unlinked_externally` with DB path + remediation; fixes the previous
  NotFound→0 silent swallow.
- **Salvage checkpoint on trip** with named
  `wal_salvage_checkpoint_succeeded/failed` events.
- **Write-refusal latch**: post-trip writes fail fast (SQL_BUSY-class on
  old-domain pooled connections, SQLITE_READONLY on new-domain), reads
  continue, node stays up; fence survives monitor shutdown.
- **Wiring**: `LocalDeployment::from_parts` startup + ordered shutdown;
  final-WAL-checkpoint outcome logged at shutdown (`vks_node_server`
  filter-directive fix + `eprintln!`).
- **Frozen verify_cmd**: `scripts/live/wal-unlink-durability-repro.sh` —
  two-leg live harness (guard-on durability / guard-off detection+refusal).

Also in the PR: pre-existing clippy 1.98 fixes (toolchain drift on
channel=stable; `drain_collect` in executors, `result_large_err` ×2 in remote
routes — behavior-neutral).

## Commits

Branch `clever-pangolin` (63 commits) squash-merged into `main` via
**PR #479** as:

- `72f20ea0` — wal-unlink-durability: WAL guard + monitor, write-refusal
  latch, salvage checkpoint (#479) — merged 2026-08-31T22:50:51Z

Final ship-session commits on the branch (included in the squash):

- `1120cc54` — fix(executors,remote): pre-existing clippy 1.98 lints
- `de16b93e` — ship(wal-unlink-durability): close gate sections, re-verify
  gates, graduate docs
- `7f06257d` — ship(wal-unlink-durability): retain graduated evidence logs

## Verification

**Post-merge production verification (frozen `verify_cmd` against the exact
merge commit):** checked out `origin/main` detached at `72f20ea0`, rebuilt
`target/release/vks-node-server` from that source, ran
`SCRATCH_ROOT=/tmp/wal-prod-verify bash scripts/live/wal-unlink-durability-repro.sh`:

- **Exit 0 — 33 PASS / 0 FAIL** (Leg A 17/0, Leg B 16/0): "All tests passed"
- Leg A (SC1, incident symptom): external write session does not unlink the
  WAL; API-committed `marker-A-post` persisted through graceful stop (offline
  sqlite3 inspect)
- Leg B (SC2): fault-injection unlink detected — `wal_unlinked_externally`
  WARN naming the DB path, refusal latch armed, post-trip write rejected
  (HTTP 500), reads still served, node alive, refused write not persisted
- Offline `PRAGMA journal_mode;` = `wal` on both legs (SC3)
- `Final WAL checkpoint completed - all data flushed to main database` in the
  Leg B node log after graceful stop
- Ports 9002/9012 free afterwards; production node never touched

**Pre-merge fresh run (same session, branch HEAD `c30e2c77`,
`SCRATCH_ROOT=/tmp/wal-resume`):** identical result — exit 0, 33 PASS / 0 FAIL
(Leg A 17/0, Leg B 16/0); evidence retained at
`dev-docs/workstreams/wal-unlink-durability/plans/wal-unlink-durability/evidence/wal-040-resume*.log`.

**SC3 no-regression:** median write latency main 19ms vs branch 19ms
(0% delta, <10% cliff).

**Mandatory gate (stable 1.98.0):** `cargo clippy --all --all-targets
--all-features -- -D warnings` EXIT 0; `cargo test --workspace` EXIT 0 (68
suites); frontend lint/tsc EXIT 0; remote-frontend lint/tsc/vitest EXIT 0 (413
tests; Node 26 needs `NODE_OPTIONS=--no-experimental-webstorage` — backlog
F-2026-08-31-03, CI pins Node 22).

## Tracked follow-ups (backlog)

- `F-2026-08-31-01` (low): WAL write refusal surfaces as generic HTTP 500 —
  wants a refusal-specific ApiError variant + harness leg-B assertion.
- `F-2026-08-31-02` (low): residual write-refusal races span new and old WAL
  domains — wants a central DB write gate.
- `F-2026-08-31-03` (low): remote-frontend vitest on Node 26 (engines range
  admits Node versions the suite doesn't run green on).

Full spec, decisions-ledger (incl. Reachability gate + Deploy verification
sections), panel history, and evidence:
`dev-docs/workstreams/wal-unlink-durability/`.

## Document verification

Verified 2026-08-31 against the pushed `main`:

- **Rendering**: GitHub blob page renders correctly — YAML frontmatter as a
  metadata table, H1 + four H2 sections with anchors, all lists/code spans
  well-formed.
- **Merge commit**: `72f20ea031072f0a2edfba5dd484e243cf27c170` on `main`.
- **PR status**: #479 **MERGED** (squash, 2026-08-31T22:50:51Z).
- **Branch commits** `1120cc54` / `de16b93e` / `7f06257d` / `c30e2c77`: resolve
  via PR #479 commit history (head branch `clever-pangolin` was auto-deleted
  on merge; SHAs remain reachable through the PR and local clones).
- **Follow-ups**: `F-2026-08-31-01` / `-02` / `-03` all present in
  `dev-docs/BACKLOG.md` on `main`.
- **Referenced paths**: spec, decisions-ledger, `wal-040-resume*.log` evidence,
  repro script, `wal_guard.rs`, `wal_monitor.rs` — all exist on `main`.

VERDICT: PASS — no dangling references.
