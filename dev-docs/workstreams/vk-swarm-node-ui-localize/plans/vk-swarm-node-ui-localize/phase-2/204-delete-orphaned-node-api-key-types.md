---
id: "204"
phase: 2
title: "Delete the four node API-key/merge type declarations orphaned by tasks 201-202"
status: passed
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - frontend/src/types/nodes.ts
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC3]
---

## Why this task exists (created mid-run, not at decomposition)

Task 202's adversarial panel found a gap no gate could catch. After 201 deleted the UI and 202
deleted the client methods, four type declarations in `frontend/src/types/nodes.ts` became a closed
cluster with **zero consumers anywhere in `frontend/src`**, and the panel confirmed by grepping every
remaining task file in phases 2-5 that **no later task claimed them**.

They break nothing: because they are `export`ed, neither `noUnusedLocals`
(`frontend/tsconfig.json:16`) nor ESLint's no-unused rules fire, so `tsc` and `lint` both stay green.
That is precisely the danger — the workstream would have closed fully green while shipping dead code
that the spec's decision D3 says should be gone.

Deleting them was correctly OUT of task 202's scope (`types/nodes.ts` is not in its `files:`), so
this is a new task rather than a scope stretch. Created in THIS session per CLAUDE.md
"No Deferred Remediation".

## Failing test (write first)

N/A — deletion of dead type declarations. Coverage is `tsc --noEmit` (which WILL fail loudly if any
consumer actually exists), plus the greps in Manual verification and the vitest baseline delta.

**`forbid_after` is deliberately NOT set.** `NodeApiKey` and `MergeNodesResponse` have legitimate
survivors outside `frontend/`: the hive's `remote-frontend/src/types/nodes.ts` and its Rust
counterparts in `crates/remote/`, all of which must survive (SC7).

## Pre-dispatch verification (ORCHESTRATOR — already done, recorded so you need not redo it)

`grep -rn "\b<name>\b" frontend/src | grep -v 'types/nodes.ts'` returns NOTHING for all four names.
The only intra-file reference is `CreateNodeApiKeyResponse.api_key: NodeApiKey`, which disappears
with the block.

## Change

- **File:** `frontend/src/types/nodes.ts`
- **Action:** delete these four `export interface` declarations in full, including their doc
  comments and the `/** Response from merging two nodes */` comment above `MergeNodesResponse`:

| Declaration | Line (at time of writing) |
|---|---|
| `NodeApiKey` | 46 |
| `CreateNodeApiKeyRequest` | 67 |
| `CreateNodeApiKeyResponse` | 72 |
| `MergeNodesResponse` | 78 |

`NodeApiKey` through `MergeNodesResponse` are contiguous at the end of the file. Delete the whole
run, leaving the file ending after the declaration that precedes `NodeApiKey`.

**Do NOT touch any other declaration in this file** — `Node`, `NodeProject`, and anything else there
have live consumers (`frontend/src/lib/api/nodes.ts`, `pages/Nodes.tsx`, `hooks/useNode.ts`,
`components/swarm/NodeProjectsSection.tsx`).

## Allowed moves

- Delete exactly those four interfaces from `frontend/src/types/nodes.ts`. Nothing else, in no other
  file.

## STOP triggers

- If `tsc --noEmit` reports ANY missing-symbol error after the deletion, a consumer exists that the
  pre-dispatch grep missed. STOP and report it — do NOT delete the consumer.
- If any of the four is re-exported from a barrel file (e.g. an `index.ts`) that other code imports
  from, STOP.
- **Do NOT touch `remote-frontend/`.** The hive has its own copies which must survive (SC7).
- If the line numbers have drifted, locate the declarations by name; if a name is absent, STOP.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cd frontend && npx tsc --noEmit
# Expected: exit 0, no output. This is the load-bearing gate — a real consumer would fail here.

cd frontend && npm run lint
# Expected: exit 0

cd frontend && npx vitest run
# Expected: the SAME 8 failing files / 15 failing tests as the documented baseline. Any new
# failure or count change is a STOP. (Baseline: Test Files 8 failed | 26 passed (34),
# Tests 15 failed | 408 passed (423) — F-2026-07-31-01..03.)

grep -rn 'NodeApiKey\|CreateNodeApiKeyRequest\|CreateNodeApiKeyResponse\|MergeNodesResponse' frontend/src
# Expected: NO output

ls remote-frontend/src/types/nodes.ts
# Expected: still exists — the hive's copy is untouched (SC7)
```

## Done when

- The four interfaces no longer exist in `frontend/src/types/nodes.ts`.
- `grep -rn` for all four names across `frontend/src` returns nothing.
- `remote-frontend/` is untouched.
- `tsc --noEmit` and `lint` are exit 0; the vitest failing set is byte-identical to baseline.
