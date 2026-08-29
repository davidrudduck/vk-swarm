---
id: "002"
phase: 1
title: "T1 evidence: pin unlink mechanism, pick guard mode, verdict A6 salvage"
status: ready
depends_on: ["001"]
parallel: false
conflicts_with: ["001","040"]
files:
  - "docs/plans/wal-unlink-durability/decisions-ledger.md"
  - "scripts/live/wal-unlink-durability-repro.sh"
irreversible: false
scope_test: "N/A"
allowed_change: mixed
red_proof: After SCRIPT MAINTENANCE encodes the write-session stimulus, the script exits non-zero on current code: leg B trip detector fires, no named events, the post-trip write is accepted-but-absent offline (marker-B-post offline count = 0), and leg A's red arm fires under the same encoded stimulus (marker-A-post offline count = 0).
covers_criteria: ["SC4"]
covers_tests: ["TS4"]
---
## Failing test (write first)
N/A — investigation task; deliverable is ledger evidence. Gate env: WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="true" WAI_LINT_CMD="true".


## Change
Append a `## T1 mechanism evidence (2026-08-30)` section to docs/plans/wal-unlink-durability/decisions-ledger.md recording three empirical verdicts, each produced against the :9012 scratch-node pattern using task 001's script and leg helpers (re-run legs via `LEGS=A bash scripts/live/wal-unlink-durability-repro.sh` etc. — NEVER source the script: it is `set -euo pipefail` with an unguarded `main` and would run the whole harness in your shell; never touch :9002).

Amendment 2026-08-30 (operator-approved re-plan): the trip stimulus is the CONFIRMED external **write session** (the 2026-08-30 evidence already established: read-only flow not reproducible under any probed condition; an external write session reliably yields `db.sqlite-wal (deleted)` in the node's fd table). The vector hunt is OVER — what remains is the three design-decision experiments below, all vector-agnostic. Do NOT re-probe the read vector.

VERDICT 1 — Unlinker identity + mechanism paragraph (under the write-vector stimulus). While a scratch node runs with a live WAL, execute an external write session (e.g. `sqlite3 <db> 'PRAGMA user_version=1;'` — a minimal write that opens, writes, closes cleanly, leaving no schema artifact) and capture: `ls -l /proc/<pid>/fd | grep -E 'sqlite|wal'` before/after; `lslocks | grep -i sqlite` if available; and `strace -f -e trace=unlinkat,renameat2 sqlite3 <db> '<write>'` on the write-session invocation itself. State precisely: which process unlinks db.sqlite-wal/-shm, under what lock state (the A5 model probe showed a connection holding the wal-index BLOCKS the unlink — so name the window the write session opens that the read session does not), and cite the 2026-08-30 read-vs-write evidence already in the ledger (`## T1 mechanism evidence (2026-08-30)`, the VERDICT 1 prose — do not re-run it).

VERDICT 2 — Guard mode (spec D4 gate). Run the incident flow (001 leg-A shape, write-session stimulus) on fresh scratch DBs, TWO runs per arm, with a throwaway PYTHON probe process standing in for the not-yet-built WalGuard: (a) control: no probe (expect red — post-trip write lost; a GREEN control arm INVALIDATES that arm set — re-run; a mode verdict may only be recorded against a control that actually tripped); (b) MapOnly probe: `python3 -c 'import sqlite3,time,sys; c=sqlite3.connect(sys.argv[1]); c.execute("PRAGMA journal_mode=WAL"); c.execute("SELECT count(*) FROM tasks").fetchall(); time.sleep(600)' <db>` (connection open, wal-index mapped, no held txn); (c) HoldRead probe: same but with `sqlite3.connect(sys.argv[1], isolation_level=None)` and `c.execute("BEGIN DEFERRED"); c.execute("SELECT count(*) FROM tasks").fetchall()` then sleep (held read txn keeps a read-mark on the wal-index). PROBE LIFECYCLE: start the probe only after boot-2 is healthy (migrations applied, tasks table exists); capture its exact PID; kill it (exact PID) BEFORE the graceful stop + offline inspect — a live probe is itself an external connection whose close can checkpoint/unlink and pollute the offline read. Record per arm BOTH metrics: (i) does `db.sqlite-wal (deleted)` appear in `/proc/<node-pid>/fd` (the unlink itself), and (ii) does the offline marker row survive. Verdict names the mode WalGuard implements — MapOnly or HoldRead — on BOTH metrics (a mode that preserves the marker but not the WAL still fails leg A's guard assertion). If BOTH probes still lose the write, that is spec decision point DP2 — STOP with halt code human_gate_required; the guard premise A5 is refuted on the real node, so do NOT proceed to Phase 2.

VERDICT 3 — A6 salvage viability. On a tripped scratch node (guard-off boot, write-session trip), while the node still runs with the unlinked WAL open, run a checkpoint from a SECOND surviving connection (python probe connected before the trip, kept open): `c.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchall()`; then offline-inspect whether pre-trip committed frames landed in the main DB. Record: checkpoint-via-surviving-open-fd does / does not flush old-inode frames. If it does NOT, salvage degrades to named-failure + refusal (US2 still met) — record that explicitly so task 030 encodes the right assertion.

Also record: whether the HoldRead probe blocked a stand-in second connection's `PRAGMA wal_checkpoint(TRUNCATE)` (it should — this is the known trade the monitor's release/reacquire coordination will exist for; the monitor itself is not built yet, so the stand-in is a second python connection), and exact host/tool versions (sqlite3 --version, python3 --version, uname -r).

SCRIPT MAINTENANCE (this task is the ONLY one besides 040 that may touch the repro script): encode the write-session stimulus from VERDICT 1 into scripts/live/wal-unlink-durability-repro.sh. FIRST re-confirm the exact write shape from the 2026-08-30 exploratory run still trips (baseline equivalence check — a substitution failure must not be misread as environment drift); only then minimise. Requirements for the encoded step:
- SINGLE-SHOT stimulus with a CHANGING value (e.g. `PRAGMA user_version=$RANDOM` or an INSERT+DELETE on tasks), not a fixed `user_version=1` (idempotent after iteration 1 → no new WAL frame) and NOT a 30s write loop (the 2026-08-30 evidence shows an overlapping external write blocks the node's API writes — contending with leg A's own timing/marker writes would corrupt the harness; a bounded retry with backoff on contention is acceptable, ledger the choice).
- Encode it in the shared step so BOTH legs exercise it (leg A needs it for its post-fix green).
- RETIRE the INCONCLUSIVE scaffold: it exists only because the pre-002 timeout was uninformative. With a reliable trip, leg A full-mode semantics become the real contract: trip detector fires → FAIL on current code (guard absent); detector times out → PASS only as the post-fix guard-on signal. Remove the 'uninformative pre-002 timeout' message and the INCONCLUSIVE exit path.
- Rename the read-era reason strings as part of the edit (authorised): `cli_read_with_detector`, `CLI_READ_SUCCEEDED`, the 'external CLI read executed' assertion labels — to write-session equivalents.
Then re-prove the red state on current code — leg B trip detector fires, no named events, the post-trip write 'succeeds' and is absent offline — and confirm leg A's red arm (marker-A-post lost on current code) fires under the same encoded stimulus. Record the script delta in the ledger. Amendment 2026-08-29 (operator-approved re-sequence): this task now also carries 001's original red_proof + SC4/TS4 coverage. Amendment 2026-08-30 (operator-approved re-plan): the stimulus is the external write session, not the read.


## Allowed moves
Append to docs/plans/wal-unlink-durability/decisions-ledger.md AND edit scripts/live/wal-unlink-durability-repro.sh exactly as the SCRIPT MAINTENANCE paragraph describes: the shared CLI-step replacement (both legs), the INCONCLUSIVE-scaffold retirement, and the read-era reason-string renames. No other script changes (its panel-verified mechanics are settled). Run 001's script (never source it) and ad-hoc python/sqlite3 probes. Do not modify any other source file or the spec.


## STOP triggers
The write-session stimulus stops tripping — FIRST re-confirm with the exact write shape from the 2026-08-30 exploratory run; if that also fails, STOP and report (environment drift); do not substitute another vector or probe shape without operator approval. Both guard-mode probes refuted (write still lost) → STOP, halt code human_gate_required (spec DP2 — A5 refuted; do not start Phase 2). Evidence shows the backup subsystem shares the hazard → STOP, halt code human_gate_required (spec DP1 — scope renegotiation). fatrace/strace unavailable AND /proc inspection inconclusive about the unlinker → STOP and ask the operator before guessing.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
The ledger section exists and contains: (1) the unlinker identity + mechanism paragraph with /proc/fd evidence; (2) the guard-mode verdict naming MapOnly or HoldRead with the three-run transcript summary; (3) the A6 verdict with the checkpoint transcript summary. Quote each in the completion report.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 002` exits 0
