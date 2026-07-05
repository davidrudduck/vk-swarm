---
id: "203"
phase: 2
title: Port Input + Switch + Checkbox React components (TS)
status: ready
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/core/Input.tsx
  - remote-frontend/src/components/core/Switch.tsx
  - remote-frontend/src/components/core/Checkbox.tsx
  - remote-frontend/src/components/core/index.ts
  - remote-frontend/src/components/core/input-switch-checkbox.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/core/input-switch-checkbox.test.tsx"
allowed_change: mixed
covers_criteria: [SC4]
---

## Sibling alignment

Read `design-source/components/core/{Input,Switch,Checkbox}.jsx` + their `.d.ts` siblings. Input is a thin wrapper over `<input>` with a `mono` flag. Switch and Checkbox are controlled/uncontrolled toggles using `data-checked` attribute (the CSS targets `[data-checked=true]`). Preserve the controlled/uncontrolled contract and the `onCheckedChange` callback signature exactly. Record any divergence in the decisions ledger.

## Failing test (write first)

Create `remote-frontend/src/components/core/input-switch-checkbox.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Input, Switch, Checkbox } from './index';

describe('Input (SC4)', () => {
  it('emits vks-input for defaults, vks-input--mono when mono', () => {
    const { container: c1 } = render(<Input placeholder="x" />);
    expect(c1.firstChild).toHaveClass('vks-input');
    expect(c1.firstChild).not.toHaveClass('vks-input--mono');
    const { container: c2 } = render(<Input mono placeholder="y" />);
    expect(c2.firstChild).toHaveClass('vks-input--mono');
  });

  it('passes through native input props (type, value, onChange)', () => {
    const onChange = vi.fn();
    render(<Input type="email" value="a@b" onChange={onChange} />);
    const el = screen.getByDisplayValue('a@b') as HTMLInputElement;
    expect(el.type).toBe('email');
    fireEvent.change(el, { target: { value: 'c@d' } });
    expect(onChange).toHaveBeenCalled();
  });
});

describe('Switch (SC4)', () => {
  it('renders a button[role=switch] with vks-switch + __thumb', () => {
    const { container } = render(<Switch />);
    const btn = container.querySelector('button[role="switch"]');
    expect(btn).toHaveClass('vks-switch');
    expect(container.querySelector('.vks-switch__thumb')).toBeTruthy();
  });

  it('toggles data-checked on click (uncontrolled, defaultChecked)', () => {
    const { container } = render(<Switch defaultChecked />);
    const btn = container.querySelector('button[role="switch"]') as HTMLButtonElement;
    expect(btn.dataset.checked).toBe('true');
    fireEvent.click(btn);
    expect(btn.dataset.checked).toBe('false');
  });

  it('calls onCheckedChange on click', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(<Switch onCheckedChange={onCheckedChange} />);
    fireEvent.click(container.querySelector('button[role="switch"]')!);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });
});

describe('Checkbox (SC4)', () => {
  it('renders a button[role=checkbox] with vks-checkbox', () => {
    const { container } = render(<Checkbox />);
    expect(container.querySelector('button[role="checkbox"]')).toHaveClass('vks-checkbox');
  });

  it('toggles data-checked on click (uncontrolled, defaultChecked)', () => {
    const { container } = render(<Checkbox defaultChecked />);
    const btn = container.querySelector('button[role="checkbox"]') as HTMLButtonElement;
    expect(btn.dataset.checked).toBe('true');
    fireEvent.click(btn);
    expect(btn.dataset.checked).toBe('false');
  });

  it('calls onCheckedChange on click', () => {
    const onCheckedChange = vi.fn();
    const { container } = render(<Checkbox onCheckedChange={onCheckedChange} />);
    fireEvent.click(container.querySelector('button[role="checkbox"]')!);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it('renders the check svg (11x11 viewBox 0 0 12 12)', () => {
    const { container } = render(<Checkbox defaultChecked />);
    const svg = container.querySelector('svg');
    expect(svg).toBeTruthy();
    expect(svg?.getAttribute('viewBox')).toBe('0 0 12 12');
  });
});
```

## Change

### File: `remote-frontend/src/components/core/Input.tsx` (CREATE)
TypeScript port of `design-source/components/core/Input.jsx` (7 lines). `InputProps extends React.InputHTMLAttributes<HTMLInputElement> { mono?: boolean }`. Renders `<input className={cn('vks-input', mono && 'vks-input--mono', className)} {...props} />`.

### File: `remote-frontend/src/components/core/Switch.tsx` (CREATE)
TypeScript port of `design-source/components/core/Switch.jsx` (27 lines). Controlled/uncontrolled: if `checked` prop is provided, controlled; else internal state seeded from `defaultChecked`. `SwitchProps { checked?: boolean; defaultChecked?: boolean; onCheckedChange?: (checked: boolean) => void; disabled?: boolean; className?: string }`. Renders `<button type="button" role="switch" aria-checked={checked} data-checked={checked} className={cn('vks-switch', className)} disabled={disabled} onClick={toggle}><span className="vks-switch__thumb" /></button>`. Toggle flips state and calls `onCheckedChange`.

### File: `remote-frontend/src/components/core/Checkbox.tsx` (CREATE)
TypeScript port of `design-source/components/core/Checkbox.jsx` (29 lines). Same controlled/uncontrolled contract as Switch but `role="checkbox"`. Renders the inline check svg (11x11, viewBox `0 0 12 12`, path `M2.5 6.5l2.5 2.5 4.5-5`, stroke `currentColor`, width `2`, `round` linecaps) inside the button, with `opacity: 0` when unchecked and `opacity: 1` when checked (via the `data-checked` CSS rule in `components.css`).

### File: `remote-frontend/src/components/core/index.ts` (EDIT)
Append re-exports: `export * from './Input'; export * from './Switch'; export * from './Checkbox';` to the existing file (created in task 202).

### File: `remote-frontend/src/components/core/input-switch-checkbox.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create `Input.tsx`, `Switch.tsx`, `Checkbox.tsx` as specified.
- Append the 3 re-export lines to `remote-frontend/src/components/core/index.ts`.
- Create the `.test.tsx` file exactly as written above.
- Use `cn()` from `@/lib/utils`. Preserve `vks-*` class names + `data-checked` attribute + ARIA roles verbatim from the design source.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version (STOP on `git status` changes under `design-source/`).
- `cn()` not exported from `@/lib/utils` (task 104 drift → STOP).
- The controlled/uncontrolled contract in the JSX does not match the d.ts (record divergence in ledger; prefer the d.ts as the type contract and the JSX as the runtime contract — if they disagree, escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/core/input-switch-checkbox.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 203` exits 0.