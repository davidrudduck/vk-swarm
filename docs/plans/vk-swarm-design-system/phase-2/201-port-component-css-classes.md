---
id: "201"
phase: 2
title: Port .vks-* component CSS classes into remote-frontend
status: ready
depends_on: ["105"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/styles/components.css
  - remote-frontend/src/styles/components.test.ts
irreversible: false
scope_test: "remote-frontend/src/styles/components.test.ts"
allowed_change: create
covers_criteria: [SC1]
---

## Failing test (write first)

Create `remote-frontend/src/styles/components.test.ts`:

```ts
import { describe, it, expect } from 'vitest';

const modules = import.meta.glob('./components.css', { as: 'raw', eager: true });
const css = modules['./components.css'] as string;

describe('component CSS classes (SC1)', () => {
  it('defines the primary component classes', () => {
    for (const cls of [
      '.vks-btn', '.vks-badge', '.vks-status', '.vks-card', '.vks-input',
      '.vks-switch', '.vks-checkbox', '.vks-tabs__list', '.vks-tabs__trigger',
      '.vks-select', '.vks-loader', '.vks-task', '.vks-node', '.vks-field',
      '.vks-alert', '.vks-savebar',
    ]) {
      expect(css).toContain(cls);
    }
  });

  it('defines the button variants + sizes', () => {
    for (const cls of ['--primary', '--secondary', '--outline', '--ghost', '--destructive', '--link', '--xs', '--sm', '--md', '--lg', '--icon']) {
      expect(css).toContain(`.vks-btn${cls}`);
    }
  });

  it('defines the task status strip + node pulse keyframes', () => {
    expect(css).toContain('@keyframes vks-spin');
    expect(css).toContain('@keyframes vks-pulse');
    expect(css).toContain('.vks-task::before');
    expect(css).toContain('.vks-node__pulse');
  });

  it('defines hover/focus/active/disabled states on the button', () => {
    expect(css).toMatch(/\.vks-btn:focus-visible/);
    expect(css).toMatch(/\.vks-btn:disabled/);
    expect(css).toMatch(/\.vks-btn--primary:hover/);
  });
});
```

## Change

### File: `remote-frontend/src/styles/components.css` (CREATE)
Copy byte-for-byte from `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/components.css` (190 lines) via `cp`. The `remote-frontend/.prettierignore` was extended in task 101 to exclude `src/styles/components.css`, so the copy will not be reformatted.

The file declares all `.vks-*` component classes with their variants, sizes, `::before`/`::after` pseudo-elements, hover/focus/active/disabled states, and the two keyframes (`@keyframes vks-spin`, `@keyframes vks-pulse`).

### File: `remote-frontend/src/styles/components.test.ts` (CREATE)
Create exactly as written above.

## Allowed moves

- `cp dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/components.css remote-frontend/src/styles/components.css`.
- Create the `.test.ts` file exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9). Do NOT edit `index.css` (task 105 owns the wiring; a later task or 105-amendment will add the `@import './styles/components.css';` line — see task 208).

## STOP triggers

- `diff design-source/tokens/components.css remote-frontend/src/styles/components.css` is non-empty after `cp` (byte-identity required).
- A class the test asserts is absent from the source file → escalate.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/components.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 201` exits 0.