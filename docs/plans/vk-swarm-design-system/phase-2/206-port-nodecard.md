---
id: "206"
phase: 2
title: Port NodeCard React component (TS)
status: ready
depends_on: ["205"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/board/NodeCard.tsx
  - remote-frontend/src/components/board/index.ts
  - remote-frontend/src/components/board/nodecard.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/board/nodecard.test.tsx"
allowed_change: mixed
covers_criteria: [SC5]
---

## Sibling alignment

Read `design-source/components/board/NodeCard.jsx` + its `.d.ts` sibling. NodeCard renders a `vks-node` root with an OS glyph svg (mac/linux/windows paths), a name span, a pulse span (online→`vks-node__pulse`, offline→`vks-node__pulse--offline`), optional `meta` ReactNode, and optional `right` ReactNode. Preserve the `OS_GLYPH` map verbatim — the svg paths are the source of truth for the icons. Record any divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/components/board/nodecard.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { NodeCard } from './index';

describe('NodeCard (SC5)', () => {
  it('emits vks-node with the name in a __name span', () => {
    render(<NodeCard name="node-1" />);
    expect(screen.getByText('node-1').closest('.vks-node')).toBeTruthy();
    expect(screen.getByText('node-1')).toHaveClass('vks-node__name');
  });

  it('renders the OS glyph svg for linux', () => {
    const { container } = render(<NodeCard name="n" os="linux" />);
    expect(container.querySelector('.vks-node__os')).toBeTruthy();
    expect(container.querySelector('.vks-node__os svg')).toBeTruthy();
  });

  it('renders the online pulse when online=true', () => {
    const { container } = render(<NodeCard name="n" online />);
    expect(container.querySelector('.vks-node__pulse')).toBeTruthy();
    expect(container.querySelector('.vks-node__pulse--offline')).toBeFalsy();
  });

  it('renders the offline pulse when online=false', () => {
    const { container } = render(<NodeCard name="n" online={false} />);
    expect(container.querySelector('.vks-node__pulse--offline')).toBeTruthy();
  });

  it('renders the meta + right ReactNodes when provided', () => {
    const { container } = render(<NodeCard name="n" meta={<span data-testid="m" />} right={<span data-testid="r" />} />);
    expect(container.querySelector('[data-testid="m"]')).toBeTruthy();
    expect(container.querySelector('[data-testid="r"]')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/components/board/NodeCard.tsx` (CREATE)
TypeScript port of `design-source/components/board/NodeCard.jsx` (27 lines). `NodeOS = 'mac' | 'linux' | 'windows'`, `NodeCardProps extends React.HTMLAttributes<HTMLDivElement> { name: string; os?: NodeOS; online?: boolean; meta?: React.ReactNode; right?: React.ReactNode }`. `OS_GLYPH: Record<NodeOS, string>` maps each OS to its svg path (copy verbatim from the JSX). Renders `<div className={cn('vks-node', className)}>` + `<div className="vks-node__os"><svg ... dangerouslySetInnerHTML? or path></div>` + `<span className="vks-node__name">{name}</span>` + `<span className={online ? 'vks-node__pulse' : 'vks-node__pulse--offline'} />` + `{meta}` + `{right}`.

### File: `remote-frontend/src/components/board/index.ts` (EDIT)
Append `export * from './NodeCard';` to the existing file (created in task 205).

### File: `remote-frontend/src/components/board/nodecard.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create `NodeCard.tsx` as specified.
- Append the re-export line to `remote-frontend/src/components/board/index.ts`.
- Create the `.test.tsx` file exactly as written above.
- Use `cn()` from `@/lib/utils`. Preserve `vks-*` class names + `OS_GLYPH` paths verbatim.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- The `OS_GLYPH` paths in the port do not match the JSX byte-for-byte (the svg paths are the icon source of truth → STOP on any drift).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/board/nodecard.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 206` exits 0.