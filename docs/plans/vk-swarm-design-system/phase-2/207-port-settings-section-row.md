---
id: "207"
phase: 2
title: Port SettingsSection + SettingsRow React components (TS)
status: passed
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/settings/SettingsSection.tsx
  - remote-frontend/src/components/settings/SettingsRow.tsx
  - remote-frontend/src/components/settings/index.ts
  - remote-frontend/src/components/settings/settings.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/settings/settings.test.tsx"
allowed_change: create
covers_criteria: [SC6]
---

## Sibling alignment

Read `design-source/components/settings/{SettingsSection,SettingsRow}.jsx` + their `.d.ts` siblings. SettingsSection composes a `vks-card` with an optional icon header (`vks-settings__header` + `vks-settings__header-icon`), title, description, content body (`vks-settings__body`), and optional footer. SettingsRow composes a `vks-field` with `label`/`htmlFor`/`helper`/`error`/`inline`/`nested`/`control` props and three layout variants (stacked/inline/nested). Preserve the exact class composition and the three layout variants. Record any divergence in the ledger.

## Failing test (write first)

Create `remote-frontend/src/components/settings/settings.test.tsx`:

```tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SettingsSection, SettingsRow } from './index';

describe('SettingsSection (SC6)', () => {
  it('emits vks-card with header/title/desc/content', () => {
    const { container } = render(
      <SettingsSection title="General" description="Basics">
        <div data-testid="body" />
      </SettingsSection>
    );
    expect(container.firstChild).toHaveClass('vks-card');
    expect(container.querySelector('.vks-card__header')).toBeTruthy();
    expect(container.querySelector('.vks-card__title').textContent).toBe('General');
    expect(container.querySelector('.vks-card__desc').textContent).toBe('Basics');
    expect(container.querySelector('.vks-card__content')).toBeTruthy();
    expect(container.querySelector('[data-testid="body"]')).toBeTruthy();
  });

  it('renders the icon header when icon is provided', () => {
    const { container } = render(<SettingsSection title="T" icon={<span data-testid="i" />} />);
    expect(container.querySelector('.vks-settings__header')).toBeTruthy();
    expect(container.querySelector('.vks-settings__header-icon')).toBeTruthy();
  });

  it('renders the footer when provided', () => {
    const { container } = render(<SettingsSection title="T" footer={<div data-testid="f" />} />);
    expect(container.querySelector('.vks-card__footer')).toBeTruthy();
    expect(container.querySelector('[data-testid="f"]')).toBeTruthy();
  });
});

describe('SettingsRow (SC6)', () => {
  it('stacked layout: vks-field with label/body/error/helper', () => {
    const { container } = render(
      <SettingsRow label="Name" htmlFor="n" helper="hint" error="bad" control={<input id="n" />} />
    );
    expect(container.firstChild).toHaveClass('vks-field');
    expect(container.querySelector('.vks-field__label')).toBeTruthy();
    expect(container.querySelector('.vks-field__error').textContent).toBe('bad');
  });

  it('inline layout: vks-field--inline with __body', () => {
    const { container } = render(<SettingsRow label="X" inline control={<input />} />);
    expect(container.firstChild).toHaveClass('vks-field--inline');
    expect(container.querySelector('.vks-field__body')).toBeTruthy();
  });

  it('nested layout adds vks-field--nested', () => {
    const { container } = render(<SettingsRow label="X" nested control={<input />} />);
    expect(container.firstChild).toHaveClass('vks-field--nested');
  });

  it('renders helper when no error', () => {
    const { container } = render(<SettingsRow label="X" helper="hint" control={<input />} />);
    expect(container.querySelector('.vks-field__helper').textContent).toBe('hint');
  });
});
```

## Change

### File: `remote-frontend/src/components/settings/SettingsSection.tsx` (CREATE)
TypeScript port of `design-source/components/settings/SettingsSection.jsx` (34 lines). `SettingsSectionProps extends React.HTMLAttributes<HTMLElement> { title?: React.ReactNode; description?: React.ReactNode; icon?: React.ReactNode; footer?: React.ReactNode; contentClassName?: string }`. Renders `<section className={cn('vks-card', className)}>` + header div (`vks-card__header` + `vks-settings__header` if icon) with optional `vks-settings__header-icon` span + title h3 (`vks-card__title`) + description p (`vks-card__desc`) + content div (`vks-card__content vks-settings__body` + contentClassName) + optional footer (`vks-card__footer`).

### File: `remote-frontend/src/components/settings/SettingsRow.tsx` (CREATE)
TypeScript port of `design-source/components/settings/SettingsRow.jsx` (54 lines). `SettingsRowProps extends React.HTMLAttributes<HTMLDivElement> { label?: React.ReactNode; htmlFor?: string; helper?: React.ReactNode; error?: React.ReactNode; inline?: boolean; nested?: boolean; control?: React.ReactNode }`. Three layouts:
- **Inline:** `<div className={cn('vks-field', 'vks-field--inline', nested && 'vks-field--nested', className)}>{control}<div className="vks-field__body">{label}{helper}{error}</div></div>`
- **Stacked:** `<div className={cn('vks-field', nested && 'vks-field--nested', className)}>{label}{control}{error}{!error && helper}</div>`
Label is `<label className="vks-field__label" htmlFor={htmlFor}>`, helper `<p className="vks-field__helper">`, error `<p className="vks-field__error">`.

### File: `remote-frontend/src/components/settings/index.ts` (CREATE)
`export * from './SettingsSection'; export * from './SettingsRow';`.

### File: `remote-frontend/src/components/settings/settings.test.tsx` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 4 files as specified.
- Use `cn()` from `@/lib/utils`. Preserve `vks-*` class names verbatim.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source JSX differs from the recorded version.
- The three layout variants in the JSX do not match the d.ts contract (escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/settings/settings.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 207` exits 0.