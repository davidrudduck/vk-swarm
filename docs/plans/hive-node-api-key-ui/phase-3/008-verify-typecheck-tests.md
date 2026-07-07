---
id: "008"
phase: 3
title: Run typecheck + full vitest suite for the workstream
status: ready
depends_on: ["006", "007"]
parallel: false
conflicts_with: []
files: []
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: []
---

## Failing test (write first)

N/A — covered by existing tests added in tasks 001-007. This task is the final gate run; no new tests.

## Change

N/A — no source changes. This task is a verification run.

## Allowed moves

- Run the two commands below; record the output in the decisions-ledger task 008 entry.
- If either command fails, STOP and escalate (do not patch code in this task — the failure points to a regression in tasks 001-007 and must be fixed by amending the responsible task).
- No file edits, no commits. The commit for this workstream happens after the gate passes (per the `wai-decompose-guard.sh` flow at the end of decompose, not in this task).

## STOP triggers

- `tsc --noEmit` reports any error — the responsible task is the one whose `Done when` did not enforce the typecheck (likely 001-007; the implementer must fix the source, not this task).
- `vitest run` reports any test failure other than pre-existing failures documented in the prior ledgers (e.g. the `nodesApi` mock shape change in this workstream is not pre-existing).
- The output is not captured to the decisions-ledger (the gate's review requires evidence).

## Manual verification (record in decisions-ledger)

Run the following two commands from the worktree root. Capture the full stdout+stderr for each. Paste both outputs into the task 008 entry of `docs/plans/hive-node-api-key-ui/decisions-ledger.md`. A pass is defined as: `tsc` exits 0, and `vitest` reports `Tests  X passed (Y)` with X = all tests across the affected scopes and 0 failed.

```bash
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx src/components/swarm/index.test.tsx src/pages/Nodes.test.tsx
```

Expected scope of the vitest run:
- `src/components/swarm/NodeApiKeySection.test.tsx` — 9 tests (TS1, TS2, TS3 from task 001; TS4 from task 002; TS5, TS6 from task 003; TS7 from task 004; TS9 from task 007)
- `src/components/swarm/index.test.tsx` — 12 tests (the original 11 plus the new NodeApiKeySection smoke test from task 005)
- `src/pages/Nodes.test.tsx` — 6 tests (the original 5 plus the new TS8 test from task 006)

If any of the three files is missing from the run (e.g. the implementer added the test to a different file), record the divergence in the ledger.

## Done when

The two commands above both exit 0 AND the task 008 decisions-ledger entry contains both full outputs.
