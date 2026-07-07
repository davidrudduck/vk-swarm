---
id: "001"
phase: 1
title: Create NodeApiKeySection skeleton + list/loading/empty tests
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/NodeApiKeySection.tsx
  - remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
  - remote-frontend/src/components/swarm/index.ts
  - remote-frontend/src/components/swarm/index.test.tsx
  - remote-frontend/src/components/swarm/SwarmHealthSection.tsx
  - remote-frontend/src/components/swarm/NodeProjectsSection.tsx
  - remote-frontend/src/components/swarm/NodeTemplatesSection.tsx
  - remote-frontend/src/components/swarm/SwarmLabelsSection.tsx
  - remote-frontend/src/components/swarm/SwarmProjectsSection.tsx
  - remote-frontend/src/components/swarm/SwarmTemplatesSection.tsx
  - remote-frontend/src/components/swarm/NodeCard.tsx
  - remote-frontend/src/components/swarm/SwarmProjectRow.tsx
  - remote-frontend/src/components/swarm/MergeLabelsDialog.tsx
  - remote-frontend/src/components/swarm/MergeProjectsDialog.tsx
  - remote-frontend/src/components/swarm/MergeTemplatesDialog.tsx
  - remote-frontend/src/components/swarm/SwarmLabelDialog.tsx
  - remote-frontend/src/components/swarm/SwarmProjectDialog.tsx
  - remote-frontend/src/components/swarm/SwarmTemplateDialog.tsx
  - frontend/src/components/org/NodeApiKeySection.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: create
covers_criteria: [SC1, SC3, SC7]
covers_tests: [TS1, TS2, TS3]
---

## Failing test (write first)

Create `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` with three tests. The first two must FAIL before the component exists (the third fails because the component is not yet exported from the barrel — added in task 005). All three must pass after the component is created and uses `useQuery` against `nodesApi.listApiKeys`.

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TooltipProvider } from '@/components/ui/tooltip';
import { NodeApiKeySection } from './NodeApiKeySection';
import { nodesApi } from '@/lib/api';
import type { NodeApiKey } from '@/types/nodes';

vi.mock('@/lib/api', () => ({
  nodesApi: {
    listApiKeys: vi.fn(),
    createApiKey: vi.fn(),
    revokeApiKey: vi.fn(),
    unblockApiKey: vi.fn(),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string, options?: Record<string, unknown>) => {
      if (options && typeof options === 'object') {
        const interpolated = (fallback || key).replace(/\{\{(\w+)\}\}/g, (_, name) => String(options[name] ?? ''));
        return interpolated;
      }
      return fallback || key;
    },
    i18n: { language: 'en' },
  }),
}));

describe('NodeApiKeySection', () => {
  let queryClient: QueryClient;
  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    vi.clearAllMocks();
  });
  const renderWith = (ui: React.ReactElement) =>
    render(
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>{ui}</TooltipProvider>
      </QueryClientProvider>
    );

  it('renders without throwing when organizationId is set and query is loading (TS1)', () => {
    vi.mocked(nodesApi.listApiKeys).mockReturnValue(new Promise(() => {}));
    expect(() => renderWith(<NodeApiKeySection organizationId="org-1" />)).not.toThrow();
  });

  it('renders the empty-state copy when the query returns [] (TS2)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await waitFor(() => {
      expect(screen.getByText('settings.swarm.apiKeys.empty')).toBeInTheDocument();
    });
  });

  it('renders one ApiKeyItem per active key with name, key_prefix, bound/unbound badge, created + last-used timestamps (TS3)', async () => {
    const keys: NodeApiKey[] = [
      {
        id: 'k1', organization_id: 'org-1', name: 'MacBook', key_prefix: 'vk_abc',
        created_by: null, last_used_at: '2026-01-03T00:00:00Z', revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: 'n1', takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
      {
        id: 'k2', organization_id: 'org-1', name: 'Build', key_prefix: 'vk_xyz',
        created_by: null, last_used_at: null, revoked_at: null,
        created_at: '2026-01-02T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      },
    ];
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    await waitFor(() => {
      expect(screen.getByText('MacBook')).toBeInTheDocument();
      expect(screen.getByText('Build')).toBeInTheDocument();
      expect(screen.getByText('vk_abc')).toBeInTheDocument();
      expect(screen.getByText('vk_xyz')).toBeInTheDocument();
    });
    expect(screen.getByText('settings.swarm.apiKeys.bound')).toBeInTheDocument();
    expect(screen.getByText('settings.swarm.apiKeys.unbound')).toBeInTheDocument();
    // created + last-used timestamps (the i18n mock interpolates the `when` option as the literal ISO string)
    expect(screen.getByText(/Created 2026-01-01/)).toBeInTheDocument();
    expect(screen.getByText(/Last used 2026-01-03/)).toBeInTheDocument();
    // k2 has last_used_at = null, so the "Last used" string is NOT rendered for it
    expect(screen.queryByText(/Last used 2026-01-02/)).not.toBeInTheDocument();
  });
});
```

## Change

**Sibling-read step (mandatory, SC4).** Before authoring, read the four sibling files listed in `files:`. List every structural choice and guard in the task 001 entry of `docs/plans/hive-node-api-key-ui/decisions-ledger.md`. Justify any divergence.

For each of the two new files in this task, the create instruction:

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.tsx`
- **Anchor:** entire file (new)
- **Before:** file does not exist
- **After:** create the file with the following structural skeleton (full text is reproduced in the implementer's commit; the boundary is the list, loading, and empty states — no Dialog, no mutations):
  - Imports: `useState` from `react`; `useTranslation` from `react-i18next`; `useQuery` from `@tanstack/react-query`; `Loader2`, `Key`, `Plus` from `lucide-react`; `Button` from `@/components/ui/button`; `Card`, `CardContent`, `CardDescription`, `CardHeader`, `CardTitle` from `@/components/ui/card`; `Badge` from `@/components/ui/badge`; `TooltipProvider` from `@/components/ui/tooltip`; `formatDistanceToNow` from `date-fns`; `nodesApi` from `@/lib/api`; `NodeApiKey` type from `@/types/nodes`.
  - Default export: `export function NodeApiKeySection({ organizationId }: { organizationId: string })`.
  - Body:
    1. `const { t } = useTranslation(['settings', 'common']);`
    2. `const [showCreateDialog, setShowCreateDialog] = useState(false);` (state hook only — Dialog body is task 002)
    3. `const { data: apiKeys = [], isLoading } = useQuery({ queryKey: ['nodeApiKeys', organizationId], queryFn: () => nodesApi.listApiKeys(organizationId), enabled: !!organizationId });`
    4. `if (!organizationId) return null;` (defensive — Nodes.tsx already gates on `orgId`, but the section's TS contract says "gated only on orgId being present").
    5. Return `<TooltipProvider>` wrapping a `<Card>` (so the section is self-contained — production callers in `Nodes.tsx` do not need to provide a `TooltipProvider`). Inside the Card:
       - `<CardHeader>` containing a `<Key />` icon, `<CardTitle>{t('settings.swarm.apiKeys.title', 'Node API Keys')}</CardTitle>`, `<CardDescription>{t('settings.swarm.apiKeys.description', 'API keys allow nodes to authenticate with the hive server')}</CardDescription>`, and a "Generate API Key" `<Button onClick={() => setShowCreateDialog(true)}>` to the right (the click handler opens the dialog; the Dialog body is added in task 002).
       - `<CardContent>`:
         - Loader2 spinner when `isLoading` (matching the `SwarmHealthSection` / `NodeProjectsSection` loading pattern).
         - Empty-state `<p>` with `t('settings.swarm.apiKeys.empty', 'No API keys found. Create one to allow nodes to connect.')` when `apiKeys.length === 0`.
         - A `<div>` mapping each key to an `ApiKeyItem` row (the `ApiKeyItem` is a sub-component in the same file, defined above `NodeApiKeySection`). Each row shows: name, `key_prefix`, a `<Badge>` reading `t('settings.swarm.apiKeys.bound', 'Bound')` when `node_id` is non-null and `t('settings.swarm.apiKeys.unbound', 'Unbound')` otherwise (per spec SC3: the badge is derived from `node_id`, matching the actual `NodeApiKey.node_id` field at `remote-frontend/src/types/nodes.ts:56`), a created-timestamp string `t('settings.swarm.apiKeys.created', 'Created {{when}}', { when: formatDistanceToNow(new Date(key.created_at), { addSuffix: true }) })`, a last-used-timestamp string `t('settings.swarm.apiKeys.lastUsed', 'Last used {{when}}', { when: formatDistanceToNow(new Date(key.last_used_at), { addSuffix: true }) })` rendered ONLY when `key.last_used_at` is non-null, and a placeholder `<Button>` labeled `t('settings.swarm.apiKeys.revoke', 'Revoke')` (the click handler is a no-op — task 003 wires it).

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`
- **Anchor:** entire file (new)
- **Before:** file does not exist
- **After:** the test file shown in "Failing test (write first)" above.

**Sibling-edit step (silence the SC6 advisory; not a content change).** Do NOT modify these files; they are listed only so the lint sees them as read siblings. Reading them is recorded in the ledger. The pattern siblings (sections) shape the structural choices; the dialog siblings are listed only to document that they are NOT pattern siblings (they operate on a selected entity as a modal, not as a list/section).

- **File:** `remote-frontend/src/components/swarm/index.test.tsx` (read only — pattern sibling for the test file)
- **File:** `remote-frontend/src/components/swarm/SwarmHealthSection.tsx` (read only — pattern sibling: Card + useQuery + AlertDialog)
- **File:** `remote-frontend/src/components/swarm/NodeProjectsSection.tsx` (read only — pattern sibling: Card + useQuery + Dialog + Alert + Badge, gated on `organizationId`)
- **File:** `remote-frontend/src/components/swarm/NodeTemplatesSection.tsx` (read only — pattern sibling: Card + useQuery, gated on `organizationId`)
- **File:** `remote-frontend/src/components/swarm/SwarmLabelsSection.tsx` (read only — pattern sibling: Card + useQuery)
- **File:** `remote-frontend/src/components/swarm/SwarmProjectsSection.tsx` (read only — pattern sibling: Card + useQuery)
- **File:** `remote-frontend/src/components/swarm/SwarmTemplatesSection.tsx` (read only — pattern sibling: Card + useQuery)
- **File:** `remote-frontend/src/components/swarm/MergeLabelsDialog.tsx` (read only — NOT a pattern sibling: a modal operating on a selected entity, not a list/section; recorded in the ledger as a non-pattern reference)
- **File:** `frontend/src/components/org/NodeApiKeySection.tsx` (read only — cross-directory reference impl, the behavioral source)

## Allowed moves

- Create the two new files with exactly the structural shape above.
- Add the imports listed; do not add others.
- Do NOT add the create Dialog body, the revoke/unblock mutations, the error Alert, the show/hide toggle, or the clipboard copy logic — these are tasks 002, 003, 004.
- Do NOT add new UI primitives beyond the import list (use the existing `Card`, `Button`, `Badge` from `@/components/ui/`).
- Do NOT touch `frontend/`, the locale JSON files, `Nodes.tsx`, or the swarm barrel `index.ts` — those are tasks 005, 006, 007.
- The `ApiKeyItem` sub-component is defined in the same file (no separate file) — match the `SwarmHealthSection` precedent of a single-file component.

## STOP triggers

- The `nodesApi` mock shape in the test does not include all four functions (`listApiKeys`, `createApiKey`, `revokeApiKey`, `unblockApiKey`) — required because the lint enforces that mocks for an existing import surface include every symbol.
- The `NodeApiKeySection` component name or `organizationId` prop name does not match the test's import.
- A sibling file is missing the patterns the implementer expected (e.g. `Card`, `Badge` not in `@/components/ui/`) — escalate; do not invent a new primitive.
- The test file references a `vi.mock` for `@/lib/api` whose return shape the test does not cover — match exactly the four functions named.
  - `useTranslation` is mocked at the file level with an interpolation-aware `t(key, fallback, options)` (so the TS7 error-state test in task 004 can assert on the interpolated `{{message}}`). Divergence from the reference impl is OK; record in the ledger.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 001` exits 0.
