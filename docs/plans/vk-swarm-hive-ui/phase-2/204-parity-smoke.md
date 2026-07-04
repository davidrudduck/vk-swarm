---
id: "204"
phase: 2
title: "Rehost: parity smoke — hive swarm UI matches node swarm UI"
status: ready
depends_on: ["203"]
parallel: false
conflicts_with: []
files:
  - frontend/src/pages/Nodes.tsx
  - remote-frontend/src/pages/Nodes.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Nodes.parity.test.tsx"
allowed_change: create
covers_criteria: [SC1, SC4]
---
## Failing test (write first)
File: `remote-frontend/src/pages/Nodes.parity.test.tsx`

A parity test: render the hive `Nodes` page and the node `Nodes` page side-by-side (or in sequence) with identical mocked data (same `nodesApi.list` return, same org context mock). Assert the rendered DOM structure matches: same number of NodeCards, same section headings, same tab labels. This is a structural parity check, not pixel-perfect visual — it catches missing sections or duplicated/omitted components.

Use `@testing-library/react` `container.innerHTML` comparison (normalised whitespace) or a targeted query count (`getAllByRole('heading')` length matches, `getAllByTestId('node-card')` length matches).

## Change
- **File:** `remote-frontend/src/pages/Nodes.parity.test.tsx` (CREATE)
  - **Before:** (file does not exist)
  - **After:** the parity test described above. Import BOTH `remote-frontend/src/pages/Nodes` and (via relative path or alias) `frontend/src/pages/Nodes`. Mock the shared dependencies identically. Assert structural parity.
  - If importing across the two apps is not feasible (different tsconfig roots), fall back to two separate render snapshots and compare the key structural queries (counts + text content of headings). Record the approach in the ledger.

## Allowed moves
- Create the parity test.
- No production code changes — this task verifies the rehost.

## STOP triggers
- If parity FAILS (the hive page renders fewer sections, different component order, or missing tabs) — STOP; the rehost in 202/203 missed a component or an import. Fix the rehost, do not weaken the parity test. Record the discrepancy + fix in the ledger.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/pages/Nodes.parity.test.tsx` exits 0.
- `cd frontend && npx tsc --noEmit` exits 0 (SC4: node frontend still compiles + its Nodes page is the parity reference).
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Nodes.parity.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 204` exits 0