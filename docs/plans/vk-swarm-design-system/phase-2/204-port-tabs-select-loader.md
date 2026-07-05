---
id: "204"
phase: 2
title: Port Tabs + Select + Loader React components (TS)
status: ready
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/core/Tabs.tsx
  - remote-frontend/src/components/core/Select.tsx
  - remote-frontend/src/components/core/Loader.tsx
  - remote-frontend/src/components/core/index.ts
  - remote-frontend/src/components/core/tabs-select-loader.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/core/tabs-select-loader.test.tsx"
allowed_change: mixed
covers_criteria: [SC4]
---

## Sibling alignment

Read `design-source/components/core/{Tabs,Select,Loader}.jsx` + their `.d.ts` siblings. Tabs is a segmented control with controlled/uncontrolled value; Select is a styled native `<select>` with a chevron overlay; Loader is a spinner whose size maps to px dimensions. Preserve the `role="tablist"`/`role="tab"` ARIA roles, the `data-active` attribute, and the `vks-loader` border-top-color primary animation. Record any divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/components/core/tabs-select-loader.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Tabs, Select, Loader } from './index';

describe('Tabs (SC4)', () => {
  it('renders a vks-tabs__list[role=tablist] with one vks-tabs__trigger[role=tab] per item', () => {
    const { container } = render(<Tabs tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]} />);
    expect(container.querySelector('[role="tablist"]')).toHaveClass('vks-tabs__list');
    expect(container.querySelectorAll('[role="tab"]')).toHaveLength(2);
    expect(container.querySelectorAll('.vks-tabs__trigger')).toHaveLength(2);
  });

  it('sets data-active=true on the selected tab', () => {
    const { container } = render(<Tabs tabs={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]} value="b" />);
    const active = container.querySelector('[data-active="true"]') as HTMLButtonElement;
    expect(active).toBeTruthy();
    expect(active.textContent).toBe('B');
  });

  it('calls onValueChange on tab click', () => {
    const onValueChange = vi.fn();
    const { container } = render(<Tabs tabs={[{ value: 'a', label: 'A' }]} onValueChange={onValueChange} />);
    fireEvent.click(container.querySelector('[role="tab"]')!);
    expect(onValueChange).toHaveBeenCalledWith('a');
  });
});

describe('Select (SC4)', () => {
  it('renders a vks-select wrapper with a native select + __chevron', () => {
    const { container } = render(<Select options={[{ value: 'x', label: 'X' }]} />);
    expect(container.firstChild).toHaveClass('vks-select');
    expect(container.querySelector('select')).toBeTruthy();
    expect(container.querySelector('.vks-select__chevron')).toBeTruthy();
  });

  it('renders one option per entry', () => {
    const { container } = render(<Select options={[{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]} />);
    expect(container.querySelectorAll('option')).toHaveLength(2);
  });

  it('calls onValueChange on change', () => {
    const onValueChange = vi.fn();
    const { container } = render(<Select options={[{ value: 'a', label: 'A' }]} onValueChange={onValueChange} />);
    fireEvent.change(container.querySelector('select')!, { target: { value: 'a' } });
    expect(onValueChange).toHaveBeenCalledWith('a');
  });
});

describe('Loader (SC4)', () => {
  it('emits vks-loader with role=status', () => {
    const { container } = render(<Loader />);
    const el = container.firstChild as HTMLElement;
    expect(el).toHaveClass('vks-loader');
    expect(el.getAttribute('role')).toBe('status');
  });

  it('applies width/height for the size token', () => {
    const { container } = render(<Loader size="lg" />);
    const el = container.firstChild as HTMLElement;
    expect(el.style.width).toBe('28px');
    expect(el.style.height).toBe('28px');
  });
});
```

## Change

### File: `remote-frontend/src/components/core/Tabs.tsx` (CREATE)
TypeScript port of `design-source/components/core/Tabs.jsx` (31 lines). `TabItem { value: string; label: React.ReactNode }`, `TabsProps { tabs: TabItem[]; value?: string; defaultValue?: string; onValueChange?: (value: string) => void; className?: string }`. Controlled/uncontrolled value. Renders `<div className={cn('vks-tabs__list', className)} role="tablist">` + `tabs.map(t => <button role="tab" aria-selected={isActive} data-active={isActive} className="vks-tabs__trigger" onClick={() => select(t.value)}>{t.label}</button>)`.

### File: `remote-frontend/src/components/core/Select.tsx` (CREATE)
TypeScript port of `design-source/components/core/Select.jsx` (24 lines). `SelectOption { value: string; label: string }`, `SelectProps { options: SelectOption[]; value?: string; defaultValue?: string; onValueChange?: (value: string) => void; disabled?: boolean; className?: string }`. Renders `<div className={cn('vks-select', className)}><select ...><option value={o.value}>{o.label}</option></select><span className="vks-select__chevron"><svg .../></span></div>`. The chevron svg is the `M3 4.5L6 7.5L9 4.5` path from the JSX.

### File: `remote-frontend/src/components/core/Loader.tsx` (CREATE)
TypeScript port of `design-source/components/core/Loader.jsx` (17 lines). `LoaderProps extends React.HTMLAttributes<HTMLSpanElement> { size?: 'sm' | 'md' | 'lg' | number }`. `SIZES = { sm: 14, md: 18, lg: 28 }`. Renders `<span className={cn('vks-loader', className)} style={{ width: px, height: px, ...style }} role="status" aria-label="Loading" {...props} />`.

### File: `remote-frontend/src/components/core/index.ts` (EDIT)
Append re-exports: `export * from './Tabs'; export * from './Select'; export * from './Loader';`.

### File: `remote-frontend/src/components/core/tabs-select-loader.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create `Tabs.tsx`, `Select.tsx`, `Loader.tsx` as specified.
- Append the 3 re-export lines to `remote-frontend/src/components/core/index.ts`.
- Create the `.test.tsx` file exactly as written above.
- Use `cn()` from `@/lib/utils`. Preserve `vks-*` class names + ARIA roles + `data-active` attribute verbatim.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- `cn()` not exported from `@/lib/utils`.
- The ARIA roles in the JSX differ from the d.ts (escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/core/tabs-select-loader.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 204` exits 0.