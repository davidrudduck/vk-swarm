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
red_proof: After SCRIPT MAINTENANCE encodes the fault-injection trip, the script exits non-zero on current code: leg B trip detector fires (WAL rm'd mid-flow), no named events, the post-trip write is accepted-but-absent offline (marker-B-post offline count = 0). Leg A (external write session) is GREEN on current code AND post-fix — it is regression coverage (external sessions provably cannot unlink the WAL on this binary), not a differential arm.
covers_criteria: ["SC4"]
covers_tests: ["TS4"]
---
## Failing test (write first)
N/A — investigation task; deliverable is ledger evidence. Gate env: WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="true" WAI_LINT_CMD="true".


## Change
Append a `## T1 mechanism evidence (2026-08-30)` section to docs/plans/wal-unlink-durability/decisions-ledger.md recording three empirical verdicts, each produced against the :9012 scratch-node pattern using task 001's script and leg helpers (re-run legs via `LEGS=A bash scripts/live/wal-unlink-durability-repro.sh` etc. — NEVER source the script: it is `set -euo pipefail` with an unguarded `main` and would run the whole harness in your shell; never touch :9002).

Amendment v2 2026-08-30 (operator-approved): the vector hunt is CLOSED. Three hunts (60+ external read/write probes with lslocks captures) established that on the CURRENT binary the pool persistently holds shared POSIX locks on db + shm — no external session can become the last wal-index holder and unlink the WAL. The incident predates this behavior or hit a rare pool-replacement window. All experiments below are therefore deterministic: the trip is FAULT INJECTION (`rm` of the WAL/SHM mid-flow — the identical inode state the incident produced), and the guard-mode decision is made on lock-persistence evidence, not incident replay. Do NOT re-probe external vectors.

VERDICT 1 — Mechanism paragraph (evidence consolidation, NOT a new hunt). Re-run ONE clean capture on a scratch node: `lslocks | grep -i sqlite` + `ls -l /proc/<pid>/fd | grep -E 'sqlite|wal'` before and after an external write session (`sqlite3 <db> 'PRAGMA user_version=<random>'`), showing the node's persistent POSIX READ locks on `db.sqlite` and `db.sqlite-shm` across the session. The paragraph states: (a) on the current binary the pool holds the wal-index shared lock continuously, so external close-unlink is impossible in steady state (cite this capture + the two prior hunts in the ledger); (b) the incident's window is therefore the pool-connection-replacement gap (idle reap / conn error) or an older binary — the guard exists to hold exactly that lock through those gaps; (c) the fault-injection trip (`rm`) reproduces the incident's post-state identically (node fds show `(deleted)`).

VERDICT 2 — Guard mode (spec D4 gate), decided by LOCK-SEMANTICS. On a scratch node, run each probe for >= 60s while the node serves periodic API writes, capturing `lslocks` every 5s: (a) MapOnly probe: `python3 -c 'import sqlite3,time,sys; c=sqlite3.connect(sys.argv[1]); c.execute("PRAGMA journal_mode=WAL"); c.execute("SELECT count(*) FROM tasks").fetchall(); time.sleep(600)' <db>`; (b) HoldRead probe: `sqlite3.connect(sys.argv[1], isolation_level=None)` + `BEGIN DEFERRED` + SELECT + sleep. Record per probe: does it hold a PERSISTENT shared lock on the shm/db (the same lock class the pool holds) across the whole window, yes/no, with the lslocks transcript. ALSO record whether the HoldRead probe blocks a stand-in second connection's `PRAGMA wal_checkpoint(TRUNCATE)` (it should — the known trade the monitor's release/reacquire coordination exists for). VERDICT: the MINIMAL mode showing a persistent lock (MapOnly preferred — simpler, no checkpoint blocking; HoldRead only if MapOnly's lock proves non-persistent). If NEITHER probe shows a persistent lock → STOP with the human-gate halt code (spec DP2: the guard premise is unimplementable as designed; do not proceed to Phase 2). PROBE LIFECYCLE: start after boot-2 healthy; exact PID; kill before any offline inspect.

VERDICT 3 — A6 salvage viability (fault-injected trip). Boot a scratch node (guard-off), open a python connection BEFORE the trip and keep it open; write a marker via the node API; then `rm <db>-wal <db>-shm` (the fault injection); while the node still runs with the unlinked WAL open, run `c.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchall()` on the surviving connection; then graceful-stop and offline-inspect whether the pre-trip marker landed in the main DB. Record: checkpoint-via-surviving-open-fd does / does not flush old-inode frames. If it does NOT, salvage degrades to named-failure + refusal (US2 still met) — record that explicitly so task 030 encodes the right assertion.

Also record: whether the HoldRead probe blocked a stand-in second connection's `PRAGMA wal_checkpoint(TRUNCATE)` (it should — this is the known trade the monitor's release/reacquire coordination will exist for; the monitor itself is not built yet, so the stand-in is a second python connection), and exact host/tool versions (sqlite3 --version, python3 --version, uname -r).

SCRIPT MAINTENANCE (this task is the ONLY one besides 040 that may touch the repro script): encode the two-arm stimulus into scripts/live/wal-unlink-durability-repro.sh:
- LEG A: replace the shared CLI-read step with a SINGLE-SHOT external write session (`sqlite3 <db> "PRAGMA user_version=$RANDOM;"`) — on the current binary this provably cannot unlink the WAL (VERDICT 1), so leg A's detector timeout is the expected PASS-signal both now and post-fix; leg A's assertions become: the write session executed, the detector timed out (no trip), marker-A-post durable offline, journal_mode=wal. Leg A is regression coverage, NOT a differential arm — a TRIP in leg A is a FAIL at any time (it would mean the pool's lock behavior regressed).
- LEG B: replace the CLI-read step with the FAULT INJECTION — after marker-B-pre lands, `rm -f <legdir>/db.sqlite-wal <legdir>/db.sqlite-shm` (inode state identical to the incident: node fds show `(deleted)`), then run the trip detector — it MUST fire immediately (the fd shows (deleted) on the next poll; keep the detector polling loop as the timing-safe check rather than assuming instant visibility).
- RETIRE the INCONCLUSIVE scaffold entirely (leg A timeout is now a designed PASS, leg B trip is deterministic) — remove the INCONCLUSIVE verdict, its exit path, and the 'uninformative pre-002 timeout' message.
- Rename the read-era reason strings (authorised): `cli_read_with_detector`, `CLI_READ_SUCCEEDED`, 'external CLI read executed' labels → write-session / fault-injection equivalents.
- NO other script changes — its mechanics passed 8 adversarial panel rounds (assertion counting, sentinels, ABORTED semantics, auth-drift STOP, PID hygiene, wall-clock bounds); preserve them exactly.
Then re-prove the red state on current code per this task's red proof (leg B fires + loses the write; leg A green). Record the script delta in the ledger. Amendment 2026-08-29 (operator-approved re-sequence): this task carries 001's original red proof + SC4/TS4 coverage. Amendments 2026-08-30 (operator-approved re-plans): stimulus settled — leg A external write session (regression arm), leg B fault injection (differential arm).


## Allowed moves
Append to docs/plans/wal-unlink-durability/decisions-ledger.md AND edit scripts/live/wal-unlink-durability-repro.sh exactly as the SCRIPT MAINTENANCE paragraph describes: the shared CLI-step replacement (both legs), the INCONCLUSIVE-scaffold retirement, and the read-era reason-string renames. No other script changes (its panel-verified mechanics are settled). Run 001's script (never source it) and ad-hoc python/sqlite3 probes. Do not modify any other source file or the spec.


## STOP triggers
Neither guard-mode probe shows a persistent shared lock (MapOnly AND HoldRead both non-persistent) → STOP with the human-gate halt code (spec DP2 — the guard premise is unimplementable as designed; do not start Phase 2). The fault injection does NOT produce `db.sqlite-wal (deleted)` in the node's fd table (platform behaviour differs) → STOP and report. Evidence shows the backup subsystem shares the hazard → STOP with the human-gate halt code (spec DP1 — scope renegotiation).


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
The ledger section exists and contains: (1) the unlinker identity + mechanism paragraph with /proc/fd evidence; (2) the guard-mode verdict naming MapOnly or HoldRead with the three-run transcript summary; (3) the A6 verdict with the checkpoint transcript summary. Quote each in the completion report.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 002` exits 0
