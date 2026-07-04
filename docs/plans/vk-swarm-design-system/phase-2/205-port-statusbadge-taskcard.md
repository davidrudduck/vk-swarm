---
id: "205"
phase: 2
title: Port StatusBadge + TaskCard React components (TS)
status: ready
depends_on: ["201","202"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/board/StatusBadge.tsx
  - remote-frontend/src/components/board/TaskCard.tsx
  - remote-frontend/src/components/board/index.ts
  - remote-frontend/src/components/board/statusbadge-taskcard.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/board/statusbadge-taskcard.test.tsx"
allowed_change: create
covers_criteria: [SC5]
---

## Sibling alignment

Read `design-source/components/board/{StatusBadge,TaskCard}.jsx` + their `.d.ts` siblings. StatusBadge maps 5 statuses to labels + dot classes. TaskCard composes a `vks-task vks-task--${status}` root with a 4px left status strip (via the CSS `::before`), an optional `AttemptIndicator` (running→Loader 14px; merged/failed→inline svg 16x16), title, truncated description, and meta row (node span + label badges + days badge). Note: TaskCard uses `Badge` and `Loader` from `@/components/core` (tasks 202 + 204) — those imports must resolve via the `@/*` alias (task 100). Preserve the `vks-task--${status}` class composition and the `AttemptIndicator` states exactly. Record any divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/components/board/statusbadge-taskcard.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusBadge, TaskCard } from './index';

describe('StatusBadge (SC5)', () => {
  it('emits vks-status vks-status--todo + dot + label for defaults', () => {
    const { container } = render(<StatusBadge />);
    expect(container.firstChild).toHaveClass('vks-status');
    expect(container.firstChild).toHaveClass('vks-status--todo');
    expect(container.querySelector('.vks-status__dot')).toBeTruthy();
    expect(screen.getByText('To Do')).toBeTruthy();
  });

  it('emits the inprogress variant', () => {
    render(<StatusBadge status="inprogress" />);
    expect(screen.getByText('In Progress')).toBeTruthy();
  });

  it('hides the label when showLabel={false}', () => {
    const { container } = render(<StatusBadge status="done" showLabel={false} />);
    expect(container.firstChild).toHaveClass('vks-status--done');
    expect(container.querySelector('.vks-status__dot')).toBeTruthy();
    expect(container.textContent).toBe('');
  });

  it('uses the custom label when provided', () => {
    render(<StatusBadge status="done" label="Shipped" />);
    expect(screen.getByText('Shipped')).toBeTruthy();
  });
});

describe('TaskCard (SC5)', () => {
  it('emits vks-task vks-task--todo with the title', () => {
    render(<TaskCard title="Implement X" status="todo" />);
    const el = screen.getByText('Implement X').closest('.vks-task');
    expect(el).toHaveClass('vks-task');
    expect(el).toHaveClass('vks-task--todo');
  });

  it('renders the description when provided', () => {
    render(<TaskCard title="T" description="D" status="inprogress" />);
    expect(screen.getByText('D')).toBeTruthy();
  });

  it('renders the node span when provided', () => {
    render(<TaskCard title="T" status="done" node="node-1" />);
    expect(screen.getByText('node-1')).toBeTruthy();
  });

  it('renders up to 2 label badges + a days badge', () => {
    const { container } = render(<TaskCard title="T" status="inreview" labels={['a', 'b', 'c']} days={3} />);
    const badges = container.querySelectorAll('.vks-badge');
    expect(badges.length).toBeGreaterThanOrEqual(3);
  });

  it('renders the AttemptIndicator (running → loader, merged → svg)', () => {
    const { container: c1 } = render(<TaskCard title="T" status="inprogress" attempt="running" />);
    expect(c1.querySelector('.vks-loader')).toBeTruthy();
    const { container: c2 } = render(<TaskCard title="T" status="done" attempt="merged" />);
    expect(c2.querySelector('svg')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/components/board/StatusBadge.tsx` (CREATE)
TypeScript port of `design-source/components/board/StatusBadge.jsx` (14 lines). `TaskStatus = 'todo' | 'inprogress' | 'inreview' | 'done' | 'cancelled'`, `StatusBadgeProps extends React.HTMLAttributes<HTMLSpanElement> { status?: TaskStatus; showLabel?: boolean; label?: React.ReactNode }`. `LABELS = { todo: 'To Do', inprogress: 'In Progress', inreview: 'In Review', done: 'Done', cancelled: 'Cancelled' }`. Renders `<span className={cn('vks-status', `vks-status--${status}`, className)} {...props}><span className="vks-status__dot" />{showLabel && <span>{label ?? LABELS[status]}</span>}</span>`.

### File: `remote-frontend/src/components/board/TaskCard.tsx` (CREATE)
TypeScript port of `design-source/components/board/TaskCard.jsx` (53 lines). `TaskStatus` (re-export from StatusBadge), `AttemptState = 'running' | 'merged' | 'failed'`, `TaskCardProps extends React.HTMLAttributes<HTMLDivElement> { title: string; description?: string; status?: TaskStatus; node?: string; labels?: string[]; attempt?: AttemptState; days?: number }`. Internal `AttemptIndicator({ state })` renders Loader 14px for running, svg for merged/failed (16x16, circle + check/cross path, color success/danger). Renders `<div className={cn('vks-task', `vks-task--${status}`, className)}>` + title `<div className="vks-task__title">` + optional desc `<div className="vks-task__desc">` + meta `<div className="vks-task__meta">` with `<span className="vks-task__node">{node}</span>` + labels.slice(0,2).map(`<Badge variant="outline">`) + days Badge secondary. Imports `Badge` from `@/components/core` and `Loader` from `@/components/core`.

### File: `remote-frontend/src/components/board/index.ts` (CREATE)
`export * from './StatusBadge'; export * from './TaskCard';`.

### File: `remote-frontend/src/components/board/statusbadge-taskcard.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create `StatusBadge.tsx`, `TaskCard.tsx`, `index.ts` as specified.
- Create the `.test.tsx` file exactly as written above.
- Use `cn()` from `@/lib/utils`. Preserve `vks-*` class names verbatim. Import `Badge`/`Loader` from `@/components/core`.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- `Badge` or `Loader` not exported from `@/components/core` (tasks 202/204 drift → STOP).
- The `AttemptIndicator` states in the JSX do not match the d.ts (escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/board/statusbadge-taskcard.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 205` exits 0.