---
id: "006"
phase: 2
title: Compose NodeApiKeySection into Nodes.tsx above the node grid + extend Nodes.test.tsx (TS8)
status: ready
depends_on: ["002", "005"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pages/Nodes.tsx
  - remote-frontend/src/pages/Nodes.test.tsx
  - remote-frontend/src/pages/Tasks.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Nodes.test.tsx"
allowed_change: edit
covers_criteria: [SC1, SC7]
covers_tests: [TS8]
---

## Failing test (write first)

Append the following test to `remote-frontend/src/pages/Nodes.test.tsx`. It must FAIL before `<NodeApiKeySection />` is mounted in `Nodes.tsx` (the section is not in the JSX). It must PASS after the Change below lands.

```tsx
import { NodeApiKeySection } from '@/components/swarm/NodeApiKeySection';
// (add this import alongside the existing `@/components/swarm/NodeCard` mock)

vi.mock('@/components/swarm', async (importOriginal) => {
  const mod = await importOriginal<typeof import('@/components/swarm')>();
  return {
    ...mod,
    NodeApiKeySection: ({ organizationId }: { organizationId: string }) => (
      <div data-testid="node-api-key-section">org={organizationId}</div>
    ),
  };
});

it('renders the NodeApiKeySection when orgId is set, and omits it when orgId is undefined (TS8)', async () => {
  // Path A: orgId set
  const orgSet = { id: 'org1', name: 'Test Org', slug: 'test-org', is_personal: false, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
  vi.mocked(useOrganizations).mockReturnValue(
    createMockQuery<Organization[]>([orgSet], false, false, null)
  );
  vi.mocked(nodesApi.list).mockResolvedValue([]);
  const { unmount } = renderNodes();
  await waitFor(() => {
    expect(screen.getByTestId('node-api-key-section')).toBeInTheDocument();
    expect(screen.getByTestId('node-api-key-section').textContent).toBe('org=org1');
  });
  unmount();

  // Path B: no orgId — section omitted
  vi.mocked(useOrganizations).mockReturnValue(
    createMockQuery<Organization[]>([], false, false, null)
  );
  renderNodes();
  expect(screen.queryByTestId('node-api-key-section')).not.toBeInTheDocument();
});
```

## Change

- **File:** `remote-frontend/src/pages/Nodes.tsx`
- **Anchor:**
  - Imports block (line 3, alongside `NodeCard` import)
  - JSX (line 27-49: the `<div className="space-y-6 …">` body) — add the section above the loading/no-org/error/empty/list branch
- **Before:** (existing imports, line 1-5)
  ```tsx
  import { useQuery } from '@tanstack/react-query';
  import { Loader2 } from 'lucide-react';
  import { NodeCard } from '@/components/swarm/NodeCard';
  import { nodesApi } from '@/lib/api';
  import { useOrganizations } from '@/hooks/useOrganizations';
  ```
- **After:**
  ```tsx
  import { useQuery } from '@tanstack/react-query';
  import { Loader2 } from 'lucide-react';
  import { NodeApiKeySection } from '@/components/swarm';
  import { NodeCard } from '@/components/swarm/NodeCard';
  import { nodesApi } from '@/lib/api';
  import { useOrganizations } from '@/hooks/useOrganizations';
  ```
  The new import uses the barrel path (`@/components/swarm`) to match the plan's "single import from `@/components/swarm`" convention. `NodeCard` is imported from the direct path because the barrel re-exports it but using the direct path keeps the existing convention of explicit subpath imports for NodeCard.

- **Before:** (the existing JSX, line 26-50)
  ```tsx
  return (
    <div className="space-y-6 p-8 pb-16 md:pb-8 h-full overflow-auto">
      <h2 className="font-serif text-2xl font-semibold">Nodes</h2>

      {isLoading ? (
        ...
      ) : !orgId ? (
        <p className="text-muted-foreground">
          Nodes are a swarm feature. Connect a hive server to get started.
        </p>
      ) : isError ? (
        <p className="text-muted-foreground">Failed to load nodes.</p>
      ) : nodes.length === 0 ? (
        <p className="text-muted-foreground">No nodes connected yet.</p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3 max-w-[1000px]">
          {nodes.map((node) => (
            <NodeCard key={node.id} node={node} />
          ))}
        </div>
      )}
    </div>
  );
  ```
- **After:** (mount `<NodeApiKeySection />` ABOVE the existing branch, gated on `orgId` being defined — per spec SC1: "gated only on `orgId` being present". The existing `!orgId` branch (which shows "Nodes are a swarm feature…") still wins for the no-org case; the section is omitted in that case because no org means no API keys to manage.)
  ```tsx
  return (
    <div className="space-y-6 p-8 pb-16 md:pb-8 h-full overflow-auto">
      <h2 className="font-serif text-2xl font-semibold">Nodes</h2>

      {orgId && <NodeApiKeySection organizationId={orgId} />}

      {isLoading ? (
        ...
      ) : !orgId ? (
        <p className="text-muted-foreground">
          Nodes are a swarm feature. Connect a hive server to get started.
        </p>
      ) : isError ? (
        ...
      ```
  The `…` ellipses are the existing bodies; do not modify them.

- **File:** `remote-frontend/src/pages/Nodes.test.tsx`
- **Anchor:**
  - Imports block (after line 22): add `NodeApiKeySection` import + a `vi.mock` for it.
  - `describe('Nodes', ...)` block: append the new `it(...)` from "Failing test (write first" above.

- **File:** `remote-frontend/src/pages/Tasks.tsx` (read only — listed in `files:` to silence the SC4 cross-directory sibling advisory). No content change.

## Allowed moves

- Edit `Nodes.tsx` to add the import and the single JSX line `{orgId && <NodeApiKeySection organizationId={orgId} />}`.
- Edit `Nodes.test.tsx` to add the mock and the TS8 test. No other test additions.
- Do NOT touch `Tasks.tsx` — it is read only.
- Do NOT touch the swarm components, the locale JSON, or the barrel — those are tasks 001-005, 007.

## STOP triggers

- The `NodeApiKeySection` is mounted below the loading/no-org branch (must be above, per spec Decision 2: "Render the section ABOVE the node grid").
- The section is mounted with a non-string `orgId` (e.g. `organizationId={orgId!}`) — the section's TS contract requires a non-empty string; the `orgId && <NodeApiKeySection />` guard is mandatory.
- The existing `!orgId` branch is removed — it must stay (the test in TS8 path B asserts that the section is omitted when `orgId` is undefined, which only makes sense if the `!orgId` branch still shows the "Connect a hive server" copy).
- The mock for `@/components/swarm` returns `null` for `NodeApiKeySection` (must return a `<div data-testid="node-api-key-section">` so the test's `getByTestId` can find it).
- The new test is placed inside the `describe('Nodes', ...)` block AFTER an existing `it(...)` that uses `renderNodes` without `unmount` — the test's `unmount()` call between paths is required to reset the DOM.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Nodes.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 006` exits 0.
