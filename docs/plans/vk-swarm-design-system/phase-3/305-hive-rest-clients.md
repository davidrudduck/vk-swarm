---
id: "305"
phase: 3
title: Hive REST clients (nodes/tasks/labels) — bare JSON, Bearer auth
status: ready
depends_on: ["102"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/lib/api/nodes.ts
  - remote-frontend/src/lib/api/tasks.ts
  - remote-frontend/src/lib/api/swarmLabels.ts
  - remote-frontend/src/lib/api/organizations.ts
  - remote-frontend/src/lib/api/swarmProjects.ts
  - remote-frontend/src/lib/api/rest.test.ts
irreversible: false
scope_test: "remote-frontend/src/lib/api/rest.test.ts"
allowed_change: create
covers_criteria: [SC8]
---

## Sibling alignment

Read `remote-frontend/src/lib/api/{utils,profile,oauth}.ts` (tasks 102). Established contract: hive returns BARE `Json(...)` (no `ApiResponse::success` envelope); `utils.ts` exports `ApiError`, `makeRequest`, `anySignal`, `REQUEST_TIMEOUT_MS`, `ApiResponse<T>` type; `profile.ts` + `oauth.ts` use `makeRequest` then `if (!response.ok) throw new ApiError(...); return await response.json() as T;`. All hive routes nest under `/v1` (`crates/remote/src/routes/mod.rs:112-113`). Bearer token from `localStorage.getItem('access_token')` (`crates/remote/src/auth/middleware.rs:46-53` — Bearer-only, no cookie). Follow the SAME pattern: `makeRequest` → check `response.ok` → `response.json() as T`. Do NOT use `handleApiResponse` (it was removed in task 102 r2 — hive is bare JSON).

## Failing test (write first)

Create `remote-frontend/src/lib/api/rest.test.ts`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { nodesApi } from './nodes';
import { tasksApi } from './tasks';
import { swarmLabelsApi } from './swarmLabels';

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: true, json: async () => ({}) } as Response);
});

describe('nodesApi (SC8)', () => {
  it('list(orgId) GETs /v1/nodes?organization_id= with Bearer header', async () => {
    await nodesApi.list('org-1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/nodes?organization_id=org-1');
    expect((init.headers as Record<string,string>).Authorization).toBe('Bearer test-token');
  });
});

describe('tasksApi (SC8)', () => {
  it('bulk(projectId) GETs /v1/tasks/bulk?project_id= with Bearer header', async () => {
    await tasksApi.bulk('proj-1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/tasks/bulk?project_id=proj-1');
    expect((init.headers as Record<string,string>).Authorization).toBe('Bearer test-token');
  });
  it('setExecutingNode(id, nodeId) PATCHes /v1/tasks/{id}/executing-node with {node_id}', async () => {
    await tasksApi.setExecutingNode('t1', 'n1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/tasks/t1/executing-node');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ node_id: 'n1' });
  });
});

describe('swarmLabelsApi (SC8)', () => {
  it('list(orgId) GETs /v1/swarm/labels?organization_id= with Bearer header', async () => {
    await swarmLabelsApi.list('org-1');
    const [url, init] = (globalThis.fetch as any).mock.calls[0];
    expect(url).toContain('/v1/swarm/labels?organization_id=org-1');
    expect((init.headers as Record<string,string>).Authorization).toBe('Bearer test-token');
  });
});
```

## Change

### File: `remote-frontend/src/lib/api/nodes.ts` (CREATE)
```ts
import { makeRequest, ApiError } from './utils';
const API_BASE = import.meta.env.VITE_API_BASE_URL || '';
function authHeaders() {
  const t = localStorage.getItem('access_token');
  return t ? { Authorization: `Bearer ${t}` } : {};
}
export interface Node { id: string; name: string; os_info: string | null; status: string; last_heartbeat_at: string | null; hostname: string | null; public_url: string | null; }
export const nodesApi = {
  async list(organizationId: string): Promise<Node[]> { const r = await makeRequest(`${API_BASE}/v1/nodes?organization_id=${encodeURIComponent(organizationId)}`, { headers: authHeaders() }); if (!r.ok) { const body = await r.text(); throw new ApiError(body || 'Request failed', r.status, r); } return await r.json() as Node[]; },
  async get(id: string): Promise<Node> { const r = await makeRequest(`${API_BASE}/v1/nodes/${id}`, { headers: authHeaders() }); if (!r.ok) { const body = await r.text(); throw new ApiError(body || 'Request failed', r.status, r); } return await r.json() as Node; },
  async remove(id: string): Promise<void> { const r = await makeRequest(`${API_BASE}/v1/nodes/${id}`, { method: 'DELETE', headers: authHeaders() }); if (!r.ok) { const body = await r.text(); throw new ApiError(body || 'Request failed', r.status, r); } },
};
```

### File: `remote-frontend/src/lib/api/tasks.ts` (CREATE)
Same pattern. `tasksApi.bulk(projectId: string) → GET /v1/tasks/bulk?project_id=${encodeURIComponent(projectId)}` returning `BulkSharedTasksResponse { tasks: Task[], deleted_task_ids: string[], latest_seq: number }` (per `crates/remote/src/routes/tasks.rs:50-64,654-659`). `tasksApi.get(id) → GET /v1/tasks/{id}`, `tasksApi.setExecutingNode(id, nodeId) → PATCH /v1/tasks/{id}/executing-node body {node_id}` (per `:620-652`), `tasksApi.assign(id, newAssigneeUserId, version?) → POST /v1/tasks/{id}/assign body {new_assignee_user_id, version}` (per `:703-706`). Export `Task` + `BulkSharedTasksResponse` interfaces. Use `ApiError(body || 'Request failed', r.status, r)` (NOT `ApiError(r.status, await r.text())` — the constructor signature is `(message, statusCode?, response?, error_data?)` per `utils.ts:20-25`).

### File: `remote-frontend/src/lib/api/swarmLabels.ts` (CREATE)
Same pattern. `swarmLabelsApi.list(organizationId: string) → GET /v1/swarm/labels?organization_id=${encodeURIComponent(organizationId)}` (per `crates/remote/src/routes/swarm_labels.rs:123-149`), `swarmLabelsApi.get(id) → GET /v1/swarm/labels/{id}`, `swarmLabelsApi.create(body) → POST /v1/swarm/labels`, `swarmLabelsApi.update(id, body) → PATCH /v1/swarm/labels/{id}`, `swarmLabelsApi.remove(id) → DELETE /v1/swarm/labels/{id}`. Export `SwarmLabel` interface. Use `ApiError(body || 'Request failed', r.status, r)`.

### File: `remote-frontend/src/lib/api/organizations.ts` (CREATE)
Same pattern. `organizationsApi.list() → GET /v1/organizations` returns `ListOrganizationsResponse { organizations: OrganizationWithRole[] }` (per `crates/remote/src/routes/organizations.rs:82-98`, `crates/utils/src/api/organizations.rs:55-58`). `OrganizationWithRole { id: string; name: string; slug: string; is_personal: boolean; created_at: string; updated_at: string; user_role: string }`. Export `OrganizationWithRole` + `ListOrganizationsResponse` interfaces.

### File: `remote-frontend/src/lib/api/swarmProjects.ts` (CREATE)
Same pattern. `swarmProjectsApi.list(organizationId: string) → GET /v1/swarm/projects?organization_id=${encodeURIComponent(organizationId)}` returns `ListSwarmProjectsResponse { projects: SwarmProjectWithNodes[] }` (per `crates/remote/src/routes/swarm_projects.rs:36-40,90-95,143-161`). Export `SwarmProjectWithNodes` + `ListSwarmProjectsResponse` interfaces. The board uses `swarmProjectsApi.list(orgId).then(r => r.projects[0]?.id)` to source a `project_id` for `tasksApi.bulk(projectId)`. If no projects exist, BoardPage renders an empty state (do NOT throw). Record the "first project" selection in the ledger.

### File: `remote-frontend/src/lib/api/rest.test.ts` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 4 files as specified.
- Reuse `makeRequest`, `ApiError` from `./utils` (task 102).
- Follow the bare-JSON pattern (no envelope unwrapping).
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `utils.ts` does not export `makeRequest` or `ApiError` (task 102 drift → STOP).
- Hive route paths differ from `crates/remote/src/routes/{nodes,tasks,swarm_labels}.rs`.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/api/rest.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 305` exits 0.