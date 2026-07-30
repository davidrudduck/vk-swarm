---
id: "202"
phase: 2
title: "Remove the unreachable API-key and merge-node methods from the node's nodesApi client"
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - frontend/src/lib/api/nodes.ts
irreversible: false
scope_test: "frontend/src/lib"
allowed_change: edit
covers_criteria: [SC3]
---

## Failing test (write first)

N/A — deletion of dead client methods. Coverage is the existing `frontend/` suite staying green
plus the scoped greps in Manual verification.

**`forbid_after` is deliberately NOT set on this task.** It greps every tracked file, and both
candidate terms have legitimate survivors: `/api/nodes/api-keys` appears in
`docs/architecture/db/functions/postgresql-node-api-keys.mdx` (updated by task 203) and in this
workstream's own spec, and `/merge-to/` appears in `crates/remote/src/routes/nodes.rs:68` and
`remote-frontend/src/lib/api/nodes.ts`, both of which are the hive's and must survive (SC7).

## Change

After task 201 nothing calls these, and after task 101 no server route answers them. Delete five
methods from `frontend/src/lib/api/nodes.ts`:

- **File:** `frontend/src/lib/api/nodes.ts`
- **Anchor:** lines 41-94 — the `listApiKeys`, `createApiKey`, `revokeApiKey`, `unblockApiKey`,
  and `mergeNodes` members of the `nodesApi` object literal (including the doc comments above
  `unblockApiKey` and `mergeNodes`)
- **After:** the object ends after `listProjects`:

```typescript
  listProjects: async (nodeId: string): Promise<NodeProject[]> => {
    const response = await makeRequest(`/api/nodes/${nodeId}/projects`);
    return handleApiResponse<NodeProject[]>(response);
  },
};
```

Then fix the import block, which now has four unused type imports:

- **Anchor:** lines 5-12
- **Before:**
```typescript
import type {
  CreateNodeApiKeyRequest,
  CreateNodeApiKeyResponse,
  MergeNodesResponse,
  Node,
  NodeApiKey,
  NodeProject,
} from '@/types/nodes';
```
- **After:**
```typescript
import type { Node, NodeProject } from '@/types/nodes';
```

## Allowed moves

- Delete the five methods and narrow the type import, in `frontend/src/lib/api/nodes.ts` only.

## STOP triggers

- If any file in `frontend/src` still references `nodesApi.listApiKeys`, `nodesApi.createApiKey`,
  `nodesApi.revokeApiKey`, `nodesApi.unblockApiKey`, or `nodesApi.mergeNodes` — that means task
  201 is incomplete. STOP; do not delete the caller from this task.
- If `Node` or `NodeProject` turns out to be unused too — that would mean `list`/`getById`/
  `delete`/`listProjects` were also removed, which this task does not authorise. STOP.
- Do NOT delete the type declarations in `frontend/src/types/nodes.ts`; they are shared and out
  of this task's `files:` list.

## Manual verification (record in decisions-ledger)

```bash
cd frontend && npx tsc --noEmit
# Expected: no output (no unused-import or missing-symbol errors)

cd frontend && npm run lint
# Expected: clean

cd frontend && npx vitest run
# Expected: all tests pass

grep -rn 'api-keys\|merge-to' frontend/src/lib/api/nodes.ts
# Expected: NO output
```

## Done when

- `nodesApi` exposes exactly `list`, `getById`, `delete`, `listProjects`.
- No `/api/nodes/api-keys` or `/merge-to/` string survives in `frontend/src/lib/api/nodes.ts`.
- `tsc --noEmit`, lint, and vitest are clean.
