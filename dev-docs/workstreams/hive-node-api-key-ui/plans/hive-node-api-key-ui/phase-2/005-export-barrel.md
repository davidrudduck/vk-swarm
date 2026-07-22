---
id: "005"
phase: 2
title: Export NodeApiKeySection from the swarm barrel + add barrel smoke test
status: ready
depends_on: ["001"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/index.ts
  - remote-frontend/src/components/swarm/index.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/index.test.tsx"
allowed_change: edit
covers_criteria: [SC1]
covers_tests: []
---

## Failing test (write first)

Append the following test to `remote-frontend/src/components/swarm/index.test.tsx`. It must FAIL before the `NodeApiKeySection` export is added to the barrel (the import `{ NodeApiKeySection }` from `./index` will be `undefined` and React will throw when rendering). It must PASS after the Change below lands.

```tsx
import { NodeApiKeySection } from './index';
// (the import line above replaces the existing import on lines 6-17)

it('renders NodeApiKeySection without throwing (smoke test from the barrel)', () => {
  expect(() => {
    renderWithProviders(<NodeApiKeySection organizationId="test-org" />);
  }).not.toThrow();
});
```

The existing `index.test.tsx` already mocks `react-i18next`, `@/lib/api` (just `nodesApi.list`), and all the swarm hooks — the new test reuses those mocks. If the implementer needs to add `nodesApi.{listApiKeys,createApiKey,revokeApiKey,unblockApiKey}` to the existing `vi.mock('@/lib/api', ...)` block, that is allowed (and required for the section to not throw on render).

## Change

- **File:** `remote-frontend/src/components/swarm/index.ts`
- **Anchor:** the section labeled "Node Card" (line 14-15) and the new "Node API Keys" section
- **Before:**
  ```ts
  // Node Card
  export { NodeCard } from './NodeCard';
  ```
- **After:**
  ```ts
  // Node Card
  export { NodeCard } from './NodeCard';

  // Node API Keys (lives on the Nodes page; not a Settings section)
  export { NodeApiKeySection } from './NodeApiKeySection';
  ```
  Place the new block immediately after the `NodeCard` export (matches the existing pattern of grouping by feature) and before the "Node Projects" block. The comment "lives on the Nodes page" is mandatory: future readers must not move this section into a Settings page (per spec Out of scope: "No new Settings page/route in `remote-frontend/`").

- **File:** `remote-frontend/src/components/swarm/index.test.tsx`
- **Anchor:**
  - The import block (lines 6-17): add `NodeApiKeySection` to the import list.
  - The `vi.mock('@/lib/api', ...)` block (lines 80-93): add `listApiKeys`, `createApiKey`, `revokeApiKey`, `unblockApiKey` to the `nodesApi` mock.
  - The `describe('Swarm Components', ...)` block: append the new `it(...)` from "Failing test (write first".
- **Before:**
  ```ts
  import {
    NodeCard,
    NodeProjectsSection,
    NodeTemplatesSection,
    SwarmHealthSection,
    SwarmLabelDialog,
    SwarmLabelsSection,
    SwarmProjectDialog,
    SwarmProjectRow,
    SwarmProjectsSection,
    SwarmTemplateDialog,
    SwarmTemplatesSection,
  } from './index';
  ```
- **After:**
  ```ts
  import {
    NodeApiKeySection,
    NodeCard,
    NodeProjectsSection,
    NodeTemplatesSection,
    SwarmHealthSection,
    SwarmLabelDialog,
    SwarmLabelsSection,
    SwarmProjectDialog,
    SwarmProjectRow,
    SwarmProjectsSection,
    SwarmTemplateDialog,
    SwarmTemplatesSection,
  } from './index';
  ```
- **Before:** (the existing `nodesApi` mock)
  ```ts
  vi.mock('@/lib/api', () => ({
    nodesApi: {
      list: vi.fn(),
    },
    ...
  }));
  ```
- **After:** (extend `nodesApi` so the new section's `useQuery` does not throw on mount)
  ```ts
  vi.mock('@/lib/api', () => ({
    nodesApi: {
      list: vi.fn(),
      listApiKeys: vi.fn().mockResolvedValue([]),
      createApiKey: vi.fn(),
      revokeApiKey: vi.fn(),
      unblockApiKey: vi.fn(),
    },
    ...
  }));
  ```

## Allowed moves

- Edit `index.ts` to add the export (one new line + one comment line).
- Edit `index.test.tsx` to extend the import, the `vi.mock` block, and append the new `it(...)`.
- Do NOT touch any other file. The section file itself is owned by tasks 001-004.

## STOP triggers

- The new export is placed under a Settings-related section header (e.g. "Swarm Settings") — the section lives on the Nodes page, not a Settings page; the comment is mandatory.
- The new export accidentally re-exports a default export from a non-existent file (must be a named re-export, matching every other line in the barrel).
- The barrel mock for `@/lib/api` removes an existing `nodesApi.list` entry — preserves the existing shape; only ADDS the four new methods.
- The new test renders without `TooltipProvider` — the existing `renderWithProviders` already includes it; reuse as-is.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/index.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 005` exits 0.
