---
id: "304"
phase: 3
title: Port TaskDrawer + DiffPanel + LogsPanel + AttemptsPanel
status: ready
depends_on: ["202","204","205","302"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/ui/panels/TaskDrawer.tsx
  - remote-frontend/src/ui/panels/DiffPanel.tsx
  - remote-frontend/src/ui/panels/LogsPanel.tsx
  - remote-frontend/src/ui/panels/AttemptsPanel.tsx
  - remote-frontend/src/ui/panels/drawer.test.tsx
  - remote-frontend/src/ui/panels/index.ts
irreversible: false
scope_test: "remote-frontend/src/ui/panels/drawer.test.tsx"
allowed_change: mixed
covers_criteria: [SC7]
---

## Sibling alignment

Read `design-source/ui_kits/vk-swarm-app/panels.jsx` (lines 44-151). `TaskDrawer({task, status, onClose})` renders an overlay + aside (460px max 90%, surface-card bg, border-left strong, shadow-lg) with header (StatusBadge + title + close button), badges row, Tabs (diff/logs/attempts), content (DiffPanel/LogsPanel/AttemptsPanel), footer (Merge/Rebase/Open in IDE buttons). `DiffPanel()` renders console-bg with lines meta/ctx/add/del. `LogsPanel({node})` renders console-bg with muted/ok/fg/cy/err lines. `AttemptsPanel()` renders rows with loader or danger dot + agent + Badge + when. The TS port replaces `window.VKSwarmDesignSystem_067861.{Button,Badge,StatusBadge,Tabs}` with direct imports. TaskDrawer accepts `task: TaskRow | null` (null → render nothing). Remove hardcoded SEED data from DiffPanel/LogsPanel/AttemptsPanel — accept `diffLines`, `logs`, `attempts` props so task 308 can wire real data. Record the prop-addition divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/ui/panels/drawer.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TaskDrawer } from './TaskDrawer';

const task = { id: 't1', title: 'Wire up OAuth callback', node: 'justX', labels: ['auth', 'backend'] };

describe('TaskDrawer (SC7)', () => {
  it('renders nothing when task is null', () => {
    const { container } = render(<TaskDrawer task={null} status="inprogress" onClose={() => {}} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders the task title + status badge + tabs', () => {
    render(<TaskDrawer task={task} status="inprogress" onClose={() => {}} />);
    expect(screen.getByText('Wire up OAuth callback')).toBeTruthy();
    expect(screen.getByText('Diff')).toBeTruthy();
    expect(screen.getByText('Logs')).toBeTruthy();
    expect(screen.getByText('Attempts')).toBeTruthy();
  });

  it('calls onClose when the close button is clicked', () => {
    const onClose = vi.fn();
    const { container } = render(<TaskDrawer task={task} status="inprogress" onClose={onClose} />);
    fireEvent.click(container.querySelector('[aria-label="Close"], .vks-btn--ghost')!);
    // The overlay also calls onClose; click the first ghost button (close)
    expect(onClose).toHaveBeenCalled();
  });

  it('renders footer Merge / Rebase / Open in IDE buttons', () => {
    render(<TaskDrawer task={task} status="inprogress" onClose={() => {}} />);
    expect(screen.getByText('Merge')).toBeTruthy();
    expect(screen.getByText('Rebase')).toBeTruthy();
    expect(screen.getByText('Open in IDE')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/ui/panels/TaskDrawer.tsx` (CREATE)
TypeScript port of `TaskDrawer` from `panels.jsx:44-91`. Props: `TaskDrawerProps { task: TaskRow | null; status: TaskStatus; onClose: () => void; diffLines?: DiffLine[]; logs?: LogLine[]; attempts?: AttemptRow[] }`. If `!task` return null. Renders overlay + aside with header (StatusBadge + h3 title + close button using `<Icon d={<>...M6 6l12 12M18 6L6 18</>} />` from `@/ui/chrome`), badges row, Tabs, content (DiffPanel/LogsPanel/AttemptsPanel — pass props or fall back to empty arrays), footer (Button primary Merge / outline Rebase / ghost Open in IDE). Import `Button`, `Badge`, `Tabs` from `@/components/core`, `StatusBadge` from `@/components/board`, `Icon` from `@/ui/chrome`.

### File: `remote-frontend/src/ui/panels/DiffPanel.tsx` (CREATE)
Port of `DiffPanel` from `panels.jsx:93-112`. Props: `DiffPanelProps { lines: DiffLine[] }` where `DiffLine = { t: 'meta' | 'ctx' | 'add' | 'del'; s: string }`. Renders console-bg div with lines mapped to colored rows.

### File: `remote-frontend/src/ui/panels/LogsPanel.tsx` (CREATE)
Port of `LogsPanel` from `panels.jsx:114-129`. Props: `LogsPanelProps { lines: LogLine[] }` where `LogLine = [kind: 'muted' | 'ok' | 'err' | 'cy' | 'fg', text: string]`. Renders console-bg div with lines mapped to colored rows.

### File: `remote-frontend/src/ui/panels/AttemptsPanel.tsx` (CREATE)
Port of `AttemptsPanel` from `panels.jsx:131-151`. Props: `AttemptsPanelProps { attempts: AttemptRow[] }` where `AttemptRow = { id: string; agent: string; state: 'running' | 'failed' | 'merged'; when: string }`. Renders rows with loader (running) or danger dot (failed) or success dot (merged), agent font-code, Badge outline state, when.

### File: `remote-frontend/src/ui/panels/index.ts` (EDIT — already exists from task 303)
Append: `export { TaskDrawer } from './TaskDrawer'; export { DiffPanel } from './DiffPanel'; export { LogsPanel } from './LogsPanel'; export { AttemptsPanel } from './AttemptsPanel'; export type { TaskRow, DiffLine, LogLine, AttemptRow } from './types';` — create `types.ts` if needed OR inline types in each component and re-export.

### File: `remote-frontend/src/ui/panels/drawer.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 5 new files + edit index.ts.
- Import design-system components from `@/components/core`, `@/components/board`, `@/ui/chrome`.
- Preserve the inline-style approach of the source JSX verbatim.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- `Tabs` not exported from `@/components/core` (task 204 drift → STOP).
- `StatusBadge` not exported from `@/components/board` (task 205 drift → STOP).
- `Icon` not exported from `@/ui/chrome` (task 302 drift → STOP).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/ui/panels/drawer.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 304` exits 0.