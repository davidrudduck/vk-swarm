---
id: "040"
phase: 5
title: "Ship gate: verify_cmd green on both legs + SC3 no-regression evidence"
status: ready
depends_on: ["001","022"]
parallel: false
conflicts_with: ["002"]
files:
  - "docs/plans/wal-unlink-durability/decisions-ledger.md"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: ["SC3","SC4"]
covers_tests: ["TS4"]
---
## Failing test (write first)
N/A — validation task; the frozen verify_cmd IS the test. Gate env: WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="true" WAI_LINT_CMD="true".


## Change
Append a `## Ship-gate evidence (SC3)` section to docs/plans/wal-unlink-durability/decisions-ledger.md after producing all of the following:

1. GREEN RUN: `cargo build --release -p server --bin vks-node-server` then `bash scripts/live/wal-unlink-durability-repro.sh` (the spec's frozen verify_cmd). Record the exit code (must be 0) and the per-leg PASS summary: leg A (guard-on) — the external write session (2026-08-30 vector amendment) does NOT unlink the WAL (trip detector times out as designed), post-trip API write durable offline, `PRAGMA journal_mode;` prints `wal`; leg B (VK_WAL_GUARD=off) — trip detector fires, wal_unlinked_externally logged with the db path, post-trip write rejected (non-2xx or `.success==false`), wal_write_refusal_active logged BEFORE the write, node still alive.

2. SC3 TIMINGS: build a MAIN-baseline binary without the branch changes: `git worktree add --detach /data/.cache/wal-main-baseline origin/main` (--detach + origin/main: plain `main` FAILS — the branch is already checked out at /data/Code/vk-swarm and git refuses a second checkout; /data/.cache because /tmp is quota-tight for a workspace target dir). Build with the worktree's own default target dir (no CARGO_TARGET_DIR override — the worktree lives on /data, which has the space): `cargo build --release -p server --manifest-path /data/.cache/wal-main-baseline/Cargo.toml --bin vks-node-server`. Copy the binary out (`cp /data/.cache/wal-main-baseline/target/release/vks-node-server /data/.cache/wal-baseline-vks-node-server`), then `git worktree remove /data/.cache/wal-main-baseline`. Run the repro script in BASELINE mode for BOTH binaries — `MODE=baseline BINARY=/data/.cache/wal-baseline-vks-node-server bash scripts/live/wal-unlink-durability-repro.sh` and `MODE=baseline BINARY=target/release/vks-node-server bash scripts/live/wal-unlink-durability-repro.sh` — both runs must COMPLETE successfully (baseline mode carries no fixed-code assertions, so an unfixed binary passes it); collect the 5 `write_latency_ms=` samples from each timings.txt. Record median(main) vs median(branch) and the per-sample table in the ledger. A median regression over 10% is a perf cliff → that is a finding, not a pass: record it and STOP.

3. SC3 VERDICT PARAGRAPH: journal_mode observed = wal (unchanged), checkpoint/behaviour path unchanged in the no-external-access case (cite: monitor only logs/checkpoints on its pre-existing thresholds; guard adds one idle connection + one held read-mark), latency delta within tolerance → SC3 satisfied.

4. Confirm the workstream's four SCs read true end-to-end: SC1 (leg A green), SC2 (leg B green), SC3 (this evidence), SC4 (the script exists and is the verify_cmd). Note anything that does NOT read true as an escalation, not a pass.


## Allowed moves
Append ONLY to docs/plans/wal-unlink-durability/decisions-ledger.md. Build binaries, run the repro script, add/remove the /data/.cache/wal-main-baseline worktree. Do not modify the repro script (if it is wrong, that is a finding against task 002 — escalate). Do not edit the spec.


## STOP triggers
The verify_cmd exits non-zero after Phase 4 → STOP; name the failing leg and assertion — this is a remediation finding against the implementation tasks, not a script tweak. Median write-latency regression > 10% → STOP and record the perf-cliff finding. Leg B's trip detector does not fire with the guard disabled → STOP; the incident mechanism has shifted and needs re-investigation (back to T1 evidence). git worktree add for the baseline fails (dirty tree/policy) → STOP and ask the operator how to obtain the baseline binary.


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
Run the frozen verify_cmd end-to-end and paste: exit code, both leg summaries, the timings table, and the journal_mode output. The ledger section must contain all four artefacts.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 040` exits 0
