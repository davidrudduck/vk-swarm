---
id: "202"
phase: 2
title: Port Button + Badge + Card React components (TS)
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/core/Button.tsx
  - remote-frontend/src/components/core/Badge.tsx
  - remote-frontend/src/components/core/Card.tsx
  - remote-frontend/src/components/core/index.ts
  - remote-frontend/src/components/core/button-badge-card.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/core/button-badge-card.test.tsx"
allowed_change: create
covers_criteria: [SC4]
---

## Sibling alignment

Read `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/components/core/{Button,Badge,Card}.jsx` + their `.d.ts` siblings. List every exclusion, guard, and structural choice each makes (e.g. Button uses a `VARIANTS`/`SIZES` map joined by string-filter; Badge appends an optional `__dot` span; Card is a compound of 6 subcomponents). The TypeScript port MUST preserve the class-name composition verbatim — use `cn()` from `@/lib/utils` (task 104) in place of the JSX's `filter(Boolean).join(' ')` pattern, but the emitted class string for a given `(variant, size, className)` triple must be byte-identical to the JSX output. Record any divergence in the decisions ledger.

## Failing test (write first)

Create `remote-frontend/src/components/core/button-badge-card.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button, Badge, Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from './index';

describe('Button (SC4)', () => {
  it('emits vks-btn + vks-btn--primary + vks-btn--md for defaults', () => {
    render(<Button>Save</Button>);
    const btn = screen.getByText('Save');
    expect(btn).toHaveClass('vks-btn');
    expect(btn).toHaveClass('vks-btn--primary');
    expect(btn).toHaveClass('vks-btn--md');
  });

  it('emits the variant + size classes for ghost/icon', () => {
    render(<Button variant="ghost" size="icon">+</Button>);
    const btn = screen.getByText('+');
    expect(btn).toHaveClass('vks-btn--ghost');
    expect(btn).toHaveClass('vks-btn--icon');
  });

  it('passes through native button props (type, disabled, onClick)', () => {
    const onClick = vi.fn();
    render(<Button type="submit" disabled onClick={onClick}>Go</Button>);
    const btn = screen.getByText('Go') as HTMLButtonElement;
    expect(btn.type).toBe('submit');
    expect(btn.disabled).toBe(true);
  });
});

describe('Badge (SC4)', () => {
  it('emits vks-badge + vks-badge--default by default', () => {
    render(<Badge>new</Badge>);
    const el = screen.getByText('new');
    expect(el).toHaveClass('vks-badge');
    expect(el).toHaveClass('vks-badge--default');
  });

  it('renders a __dot span when dot={true}', () => {
    const { container } = render(<Badge dot>live</Badge>);
    expect(container.querySelector('.vks-badge__dot')).toBeTruthy();
  });

  it('emits the destructive variant class', () => {
    render(<Badge variant="destructive">err</Badge>);
    expect(screen.getByText('err')).toHaveClass('vks-badge--destructive');
  });
});

describe('Card (SC4)', () => {
  it('renders a vks-card root with header/title/desc/content/footer subcomponents', () => {
    const { container } = render(
      <Card>
        <CardHeader>
          <CardTitle>Title</CardTitle>
          <CardDescription>Desc</CardDescription>
        </CardHeader>
        <CardContent>Body</CardContent>
        <CardFooter>Foot</CardFooter>
      </Card>
    );
    expect(container.firstChild).toHaveClass('vks-card');
    expect(container.querySelector('.vks-card__header')).toBeTruthy();
    expect(container.querySelector('.vks-card__title').tagName).toBe('H3');
    expect(container.querySelector('.vks-card__desc').tagName).toBe('P');
    expect(container.querySelector('.vks-card__content')).toBeTruthy();
    expect(container.querySelector('.vks-card__footer')).toBeTruthy();
  });
});
```

## Change

### File: `remote-frontend/src/components/core/Button.tsx` (CREATE)
TypeScript port of `design-source/components/core/Button.jsx` (32 lines). Use `cn()` from `@/lib/utils` for class composition. Export `ButtonVariant`, `ButtonSize`, `ButtonProps` types from the `.d.ts` sibling. The component signature:

```ts
import { ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

export type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'destructive' | 'link';
export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg' | 'icon';

const SIZES: Record<ButtonSize, string> = { xs: 'vks-btn--xs', sm: 'vks-btn--sm', md: 'vks-btn--md', lg: 'vks-btn--lg', icon: 'vks-btn--icon' };
const VARIANTS: Record<ButtonVariant, string> = { primary: 'vks-btn--primary', secondary: 'vks-btn--secondary', outline: 'vks-btn--outline', ghost: 'vks-btn--ghost', destructive: 'vks-btn--destructive', link: 'vks-btn--link' };

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
}

export function Button({ variant = 'primary', size = 'md', className, children, ...props }: ButtonProps) {
  const cls = cn('vks-btn', VARIANTS[variant] ?? VARIANTS.primary, SIZES[size] ?? SIZES.md, className);
  return <button className={cls} {...props}>{children}</button>;
}
```

### File: `remote-frontend/src/components/core/Badge.tsx` (CREATE)
TypeScript port of `design-source/components/core/Badge.jsx` (19 lines). `BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline'`. Renders `<span className={cn('vks-badge', `vks-badge--${variant}`, className)}>` + optional `<span className="vks-badge__dot" />` when `dot` is true.

### File: `remote-frontend/src/components/core/Card.tsx` (CREATE)
TypeScript port of `design-source/components/core/Card.jsx` (29 lines). Compound component: `Card` (`vks-card` div), `CardHeader` (`vks-card__header` div), `CardTitle` (`vks-card__title` h3), `CardDescription` (`vks-card__desc` p), `CardContent` (`vks-card__content` div), `CardFooter` (`vks-card__footer` div). Each takes `{ className, children, ...props }`.

### File: `remote-frontend/src/components/core/index.ts` (CREATE)
Re-export: `export { Button, Badge, Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from './Button';` etc. — actually export each from its own module: `export * from './Button'; export * from './Badge'; export * from './Card';`.

### File: `remote-frontend/src/components/core/button-badge-card.test.tsx` (CREATE)
Create exactly as written in Failing test above (prepend `import { vi } from 'vitest';` if `vi` is used in the Button test).

## Allowed moves

- Create the 5 files above exactly as specified.
- Use `cn()` from `@/lib/utils` (already exists from task 104) for all class composition.
- Preserve `vks-*` class names verbatim from the design source.
- No other file may be touched. Do NOT edit `frontend/` (SC9). Do NOT edit `index.css` (task 208 wires `components.css`).

## STOP triggers

- The design-source JSX file differs from the version recorded in the spec (the design-source tree is committed and immutable; if `git status` shows changes under `dev-docs/designs/.../design-source/`, STOP).
- `cn()` is not exported from `@/lib/utils` (would mean task 104 drifted → STOP, escalate).
- A class name the test asserts does not match the design-source JSX output (would mean the port diverged from the source of truth → STOP, re-read the JSX, record the divergence in the ledger if the JSX is wrong, or fix the port if the port is wrong).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/core/button-badge-card.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 202` exits 0.