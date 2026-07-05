---
id: "309"
phase: 3
title: Wire NodesView + ProcessesView to data (REST)
status: ready
depends_on: ["303","305","307"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pages/NodesPage.tsx
  - remote-frontend/src/pages/ProcessesPage.tsx
  - remote-frontend/src/pages/NodesPage.test.tsx
  - remote-frontend/src/AppRouter.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/NodesPage.test.tsx"
allowed_change: mixed
covers_criteria: [SC8]
---

## Sibling alignment

Read `remote-frontend/src/ui/panels/NodesView.tsx` + `ProcessesView.tsx` (task 303). They accept `nodes: NodeRow[]` and `processes: ProcessRow[]` props. Read `remote-frontend/src/lib/api/{nodes,organizations}.ts` (task 305). The wiring: `NodesPage` first fetches organizations via `organizationsApi.list()`, picks the first org, then fetches nodes via `nodesApi.list(orgId)` (REST, requires `organization_id` per `crates/remote/src/routes/nodes.rs:361-385`) with `useQuery`, maps `Node[]` to `NodeRow[]`, renders `<NodesView nodes={...} />`. `ProcessesView` is a placeholder — the hive does NOT have a `/processes` REST route (grep `crates/remote/src/routes/` for "process" → no route). For now, `ProcessesPage` renders `<ProcessesView processes={[]} />` (empty). Record the no-processes-route gap in the ledger.

## Failing test (write first)

Create `remote-frontend/src/pages/NodesPage.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { NodesPage } from './NodesPage';

const mockNodes = [
  { id: 'n1', name: 'justX', os_info: 'mac', status: 'online', last_heartbeat_at: '2026-07-04T10:00:00Z', hostname: 'h', public_url: 'u' },
  { id: 'n2', name: 'linux-01', os_info: 'linux', status: 'online', last_heartbeat_at: '2026-07-04T10:00:00Z', hostname: null, public_url: null },
];
const mockOrgs = { organizations: [{ id: 'org-1', name: 'Acme', slug: 'acme', is_personal: false, created_at: '', updated_at: '', user_role: 'owner' }] };

beforeEach(() => {
  localStorage.setItem('access_token', 'test-token');
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (url: any) => {
    const u = typeof url === 'string' ? url : '';
    if (u.includes('/v1/organizations')) return { ok: true, json: async () => mockOrgs } as Response;
    if (u.includes('/v1/nodes')) return { ok: true, json: async () => mockNodes } as Response;
    return { ok: true, json: async () => ({}) } as Response;
  });
});

describe('NodesPage (SC8)', () => {
  it('fetches /v1/nodes and renders NodeCards', async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={qc}><NodesPage /></QueryClientProvider>);
    await waitFor(() => {
      expect(screen.getByText('justX')).toBeTruthy();
      expect(screen.getByText('linux-01')).toBeTruthy();
    });
  });
});
```

## Change

### File: `remote-frontend/src/pages/NodesPage.tsx` (CREATE)
```tsx
import { useQuery } from '@tanstack/react-query';
import { NodesView } from '@/ui/panels';
import { nodesApi, type Node } from '@/lib/api/nodes';
import { organizationsApi } from '@/lib/api/organizations';

export function NodesPage() {
  const orgsQ = useQuery({ queryKey: ['orgs'], queryFn: organizationsApi.list });
  const orgId = orgsQ.data?.organizations[0]?.id;
  const { data: nodes = [] } = useQuery({ queryKey: ['nodes', orgId], queryFn: () => nodesApi.list(orgId!), enabled: !!orgId });
  const rows = nodes.map(mapNodeToRow);
  return <NodesView nodes={rows} />;
}

function mapNodeToRow(n: Node) {
  return {
    id: n.id,
    name: n.name,
    os: (n.os_info?.toLowerCase().includes('mac') ? 'mac' : n.os_info?.toLowerCase().includes('win') ? 'windows' : 'linux') as 'mac' | 'linux' | 'windows',
    online: n.status === 'online',
    meta: n.hostname ?? n.public_url ?? '',
    rightCount: 0,
  };
}
```

### File: `remote-frontend/src/pages/ProcessesPage.tsx` (CREATE)
```tsx
import { ProcessesView } from '@/ui/panels';
export function ProcessesPage() {
  return <ProcessesView processes={[]} />;
}
```
(The hive has no `/processes` route — empty until one is added. Record in ledger.)

### File: `remote-frontend/src/pages/NodesPage.test.tsx` (CREATE)
Create exactly as written in Failing test above.

### File: `remote-frontend/src/AppRouter.tsx` (EDIT)
Replace the `/nodes` placeholder with `<NodesPage />` (import from `./pages/NodesPage`). Add a `/processes` route (inside Chrome layout) with `<ProcessesPage />` (import from `./pages/ProcessesPage`). The Chrome Navbar's `onView('processes')` navigates here.

## Allowed moves

- Create `NodesPage.tsx` + `ProcessesPage.tsx` + `NodesPage.test.tsx`.
- Edit `AppRouter.tsx` to swap `/nodes` placeholder for `<NodesPage />` + add `/processes` route with `<ProcessesPage />`.
- Use `useQuery` from `@tanstack/react-query`.
- Map `Node` fields to `NodeRow` fields — record the exact mapping in the ledger.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `NodesView` not exported from `@/ui/panels` (task 303 drift → STOP).
- `nodesApi.list` not exported from `@/lib/api/nodes` (task 305 drift → STOP).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/NodesPage.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 309` exits 0.