---
id: "102"
phase: 1
title: Port spacing + radius + motion tokens into remote-frontend
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/styles/tokens/spacing.css
  - remote-frontend/src/styles/tokens/spacing.test.ts
irreversible: false
scope_test: "remote-frontend/src/styles/tokens/spacing.test.ts"
allowed_change: create
covers_criteria: [SC2]
---

## Failing test (write first)

Create `remote-frontend/src/styles/tokens/spacing.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const spacing = readFileSync(join(__dirname, 'spacing.css'), 'utf8');

describe('spacing tokens (SC2)', () => {
  it('defines the 4px-grid scale (--space-0..--space-16)', () => {
    for (const token of ['--space-0', '--space-1', '--space-2', '--space-3', '--space-4', '--space-5', '--space-6', '--space-8', '--space-10', '--space-12', '--space-16']) {
      expect(spacing).toContain(`${token}:`);
    }
    expect(spacing).toContain('--space-1: 0.25rem');
    expect(spacing).toContain('--space-16: 4rem');
  });

  it('defines control heights', () => {
    for (const token of ['--control-xs', '--control-sm', '--control-md', '--control-lg']) {
      expect(spacing).toContain(`${token}:`);
    }
    expect(spacing).toContain('--control-md: 2.5rem');
  });

  it('defines radius tokens', () => {
    for (const token of ['--radius-sm', '--radius-md', '--radius-lg', '--radius-xl', '--radius-full']) {
      expect(spacing).toContain(`${token}:`);
    }
  });

  it('defines border, shadow, and glow tokens', () => {
    for (const token of ['--border-width', '--strip-width', '--shadow-sm', '--shadow-md', '--shadow-lg', '--glow-cyan', '--glow-emerald']) {
      expect(spacing).toContain(`${token}:`);
    }
  });
});
```

## Change

### File: `remote-frontend/src/styles/tokens/spacing.css` (CREATE)
Copy byte-for-byte from `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/spacing.css` (46 lines) via `cp`. Declares the 4px grid (`--space-0..--space-16`), control heights (`--control-xs: 2rem` .. `--control-lg: 2.75rem`), radius (`--radius-sm: 0.25rem` .. `--radius-full: 9999px`), borders (`--border-width: 1px`, `--strip-width: 4px`), shadows (`--shadow-sm/md/lg`), and glows (`--glow-cyan`, `--glow-emerald`).

## Allowed moves

- Create `remote-frontend/src/styles/tokens/spacing.css` via `cp dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/spacing.css remote-frontend/src/styles/tokens/spacing.css`.
- Create the `.test.ts` file exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `diff design-source/tokens/spacing.css remote-frontend/src/styles/tokens/spacing.css` is non-empty after `cp` (byte-identity is required).
- A token the test asserts is absent from the source file → escalate.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/tokens/spacing.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 102` exits 0.