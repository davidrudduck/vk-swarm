---
id: "202"
phase: 2
title: "Rehost: copy swarm components + API clients + types into remote-frontend (node frontend kept as HA fallback)"
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/
  - remote-frontend/src/components/labels/
  - remote-frontend/src/components/ui/
  - remote-frontend/src/hooks/
  - remote-frontend/src/lib/api/nodes.ts
  - remote-frontend/src/lib/api/swarmProjects.ts
  - remote-frontend/src/lib/api/swarmLabels.ts
  - remote-frontend/src/lib/api/swarmTemplates.ts
  - remote-frontend/src/lib/api/templates.ts
  - remote-frontend/src/lib/api/index.ts
  - remote-frontend/src/types/nodes.ts
  - remote-frontend/src/types/swarm.ts
  - remote-frontend/src/components/swarm/index.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/index.test.tsx"
allowed_change: mixed
covers_criteria: [SC1, SC4]
---
## Failing test (write first)
File: `remote-frontend/src/components/swarm/index.test.tsx`

A compile + smoke test: import each exported component from `@/components/swarm` and render it with mocked `QueryClientProvider` + `ProfileProvider` context. Assert no throw. This is a structural test — it proves the copy + alias setup compiles and the components mount. Per-component behavioural tests are out of scope (the components are unchanged from the node frontend; the rehost is a copy, not a rewrite).

## Change
Copy the following files from `frontend/src/` to `remote-frontend/src/` (verbatim, preserving the directory structure). The `@/*` and `shared/*` aliases from task 201 make the imports resolve unchanged.

- **`frontend/src/components/swarm/*`** → **`remote-frontend/src/components/swarm/*`**
  - Copy ALL 14 component files + `index.ts`: MergeLabelsDialog, MergeProjectsDialog, MergeTemplatesDialog, NodeCard, NodeProjectsSection, NodeTemplatesSection, SwarmHealthSection, SwarmLabelDialog, SwarmLabelsSection, SwarmProjectDialog, SwarmProjectRow, SwarmProjectsSection, SwarmTemplateDialog, SwarmTemplatesSection.
  - **Sibling alignment:** the node frontend's `frontend/src/components/swarm/*` IS the sibling being copied. Read each component. List every import it makes. Confirm every import resolves under the new aliases (`@/types/*`, `@/lib/api/*`, `shared/types`, relative `./`). If a component imports from a path NOT covered by the task 201 aliases (e.g. `@/hooks/useSomething` that isn't copied yet) — STOP and copy that dependency too, recording it in the ledger. The copy must be self-contained.
  - **Known likely transitive deps (check these FIRST — Gemini F1):** swarm components typically import from `@/components/ui/*` (shadcn UI primitives — copy the subset they use), `@/lib/utils` (cn helper — copy), `@/hooks/*` (node-frontend hooks like `useUserOrganizations` — port or adapt). Before declaring the copy done, grep each component for `@/` imports and ensure every target exists under `remote-frontend/src/`. Record the full transitive closure in the ledger.

- **`frontend/src/lib/api/{nodes,swarmProjects,swarmLabels,swarmTemplates}.ts`** → **`remote-frontend/src/lib/api/`**
  - Copy these 4 files with ADAPTATION, NOT verbatim. The node API clients import and call `handleApiResponse` from `./utils`, which unwraps a `{success:boolean, data:T}` envelope. The hive returns BARE JSON (no envelope). Verified: `crates/remote/src/routes/nodes.rs:385` returns `(StatusCode::OK, Json(nodes)).into_response()` — bare `Node[]`, no `{success:true,data:[...]}` wrapper. Same for `swarm_projects.rs:147`, `swarm_labels.rs:149`, `swarm_templates.rs:151`.
  - **DO NOT copy `frontend/src/lib/api/utils.ts`** — the phase-1 `remote-frontend/src/lib/api/utils.ts` (from task 102 r2) deliberately omits `handleApiResponse`. Replacing it with the node version would re-introduce the envelope bug.
  - **Replace every `handleApiResponse<T>(response)` call** in the copied clients with the bare-JSON pattern:
    ```
    if (!response.ok) { const body = await response.text(); throw new ApiError(body || 'Request failed', response.status, response); }
    return await response.json() as T;
    ```
    This is the same pattern used in `remote-frontend/src/lib/api/oauth.ts` and `profile.ts` (task 102 r2).
  - **Remove `import { handleApiResponse, ... } from './utils'`** — change to `import { ApiError, makeRequest } from './utils'`.

- **`frontend/src/types/{nodes,swarm}.ts`** → **`remote-frontend/src/types/`**
  - Copy these type files verbatim. The swarm components + API clients import from `@/types/nodes` and `@/types/swarm`.
  - If these files import from other `@/types/*` or `shared/*`, copy those too (recurse the dependency closure).

- **API base URL adjustment (DO NOT change `/api/` — adjust to `/v1/` instead):** the node `nodesApi.list(orgId)` calls `GET /api/nodes?organization_id=...`. The hive routes nest under `/v1` (`crates/remote/src/routes/mod.rs:112-113`). Edit the copied `nodes.ts`, `swarmProjects.ts`, `swarmLabels.ts`, `swarmTemplates.ts` to use `/v1/` prefixes instead of `/api/`. Record the exact before/after for each file in the ledger.

## Allowed moves
- Create files under `remote-frontend/src/{components/swarm,components/labels,components/ui,lib/api,types,hooks}/`.
- Read-only reference to `frontend/src/{components/swarm,components/labels,components/ui,hooks,lib/api,types}/*` for the copy sources.

## STOP triggers
- If any copied component imports from a path NOT covered by `@/*`, `shared/*`, or relative — STOP, copy the missing dependency, record in ledger.
- If the hive's `/api/nodes` route prefix differs from the node frontend's assumption — STOP, fix the URL in the copied API client, record the delta.
- If a swarm component imports `useUserOrganizations` or another node-frontend-specific hook — STOP; that hook must be ported too (it's not part of the auth shell in phase 1). Copy it and record in the ledger, OR confirm it's available via the `ProfileProvider` context and adapt.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/components/swarm/index.test.tsx` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.
- `cd remote-frontend && npm run build` exits 0.
- `cd frontend && npx tsc --noEmit` exits 0 (SC4: the node frontend still compiles — the copy didn't delete anything).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/index.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 202` exits 0