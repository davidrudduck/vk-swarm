---
id: "303"
phase: 3
title: Port NodesView + ProcessesView into remote-frontend
status: ready
depends_on: ["206","202"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/ui/panels/NodesView.tsx
  - remote-frontend/src/ui/panels/ProcessesView.tsx
  - remote-frontend/src/ui/panels/index.ts
  - remote-frontend/src/ui/panels/views.test.tsx
irreversible: false
scope_test: "remote-frontend/src/ui/panels/views.test.tsx"
allowed_change: create
covers_criteria: [SC7]
---

## Sibling alignment

Read `design-source/ui_kits/vk-swarm-app/panels.jsx` (lines 1-57). `NodesView()` renders a padded div with an h2 ("Hive" in `--font-display` `--text-2xl`), a StatusBadge (done, "3 nodes online"), and a grid `repeat(auto-fill, minmax(320px, 1fr))` gap 12 maxWidth 1000 of NodeCard components with Badge right slots. `ProcessesView()` renders an h2 "Processes" + a vks-card containing rows of (loader if running else status-done dot, font-code name, font-code node, font-code dur). The TS port replaces `window.VKSwarmDesignSystem_067861.{NodeCard,Badge}` with direct imports from `@/components/board` + `@/components/core`. Remove the hardcoded SEED data from the component body — accept a `nodes` prop on NodesView and a `processes` prop on ProcessesView (so task 309 can wire real data). Record the prop-addition divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/ui/panels/views.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { NodesView, ProcessesView } from './index';

const nodes = [
  { id: 'n1', name: 'justX', os: 'mac' as const, online: true, meta: '3 agents', rightCount: 3 },
  { id: 'n2', name: 'linux-01', os: 'linux' as const, online: true, meta: '1 agent', rightCount: 1 },
  { id: 'n3', name: 'winbox', os: 'windows' as const, online: false, meta: '4m ago', rightCount: 0 },
];

const processes = [
  { id: 'p1', name: 'claude-code · feat/auth', node: 'justX', state: 'running' as const, dur: '2m 14s' },
  { id: 'p2', name: 'pnpm test', node: 'linux-01', state: 'done' as const, dur: '1m 02s' },
];

describe('NodesView (SC7)', () => {
  it('renders an h2 heading and one NodeCard per node', () => {
    render(<NodesView nodes={nodes} />);
    expect(screen.getByRole('heading', { name: 'Hive' })).toBeTruthy();
    expect(screen.getByText('justX')).toBeTruthy();
    expect(screen.getByText('linux-01')).toBeTruthy();
    expect(screen.getByText('winbox')).toBeTruthy();
  });
});

describe('ProcessesView (SC7)', () => {
  it('renders an h2 heading and one row per process', () => {
    render(<ProcessesView processes={processes} />);
    expect(screen.getByRole('heading', { name: 'Processes' })).toBeTruthy();
    expect(screen.getByText('claude-code · feat/auth')).toBeTruthy();
    expect(screen.getByText('pnpm test')).toBeTruthy();
  });
  it('renders a vks-loader for running processes', () => {
    const { container } = render(<ProcessesView processes={processes} />);
    expect(container.querySelector('.vks-loader')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/ui/panels/NodesView.tsx` (CREATE)
TypeScript port of `NodesView` from `panels.jsx:2-18`. Props: `NodesViewProps { nodes: NodeRow[] }` where `NodeRow = { id: string; name: string; os: 'mac' | 'linux' | 'windows'; online: boolean; meta: string; rightCount: number }`. Renders the padded div, h2 "Hive", a StatusBadge (done, `${nodes.filter(n => n.online).length} nodes online`), and the grid of NodeCard with `right={<Badge variant={n.online ? 'secondary' : 'outline'} dot={n.online}>{n.rightCount || 'offline'}</Badge>}`. Import `NodeCard` from `@/components/board`, `Badge` from `@/components/core`.

### File: `remote-frontend/src/ui/panels/ProcessesView.tsx` (CREATE)
TypeScript port of `ProcessesView` from `panels.jsx:20-42`. Props: `ProcessesViewProps { processes: ProcessRow[] }` where `ProcessRow = { id: string; name: string; node: string; state: 'running' | 'done'; dur: string }`. Renders h2 "Processes" + vks-card with rows: `state === 'running'` → `<span className="vks-loader" style={{ width: 14, height: 14 }} />`, else `<span className="vks-status vks-status--done"><span className="vks-status__dot" /></span>`. Then font-code name, font-code node, font-code dur.

### File: `remote-frontend/src/ui/panels/index.ts` (CREATE)
`export { NodesView } from './NodesView'; export { ProcessesView } from './ProcessesView'; export type { NodeRow, ProcessRow } from './types';` — create a `types.ts` if needed OR inline the types in each component and re-export. Preferred: inline types in each component file, re-export from index.

### File: `remote-frontend/src/ui/panels/views.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 4 files as specified.
- Import `NodeCard` from `@/components/board` (task 206), `Badge` from `@/components/core` (task 202).
- Preserve the inline-style approach of the source JSX verbatim.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- `NodeCard` not exported from `@/components/board` (task 206 drift → STOP).
- `Badge` not exported from `@/components/core` (task 202 drift → STOP).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/ui/panels/views.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 303` exits 0.