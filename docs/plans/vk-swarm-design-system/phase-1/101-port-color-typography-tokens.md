---
id: "101"
phase: 1
title: Port color + typography tokens into remote-frontend
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/styles/tokens/colors.css
  - remote-frontend/src/styles/tokens/typography.css
  - remote-frontend/src/styles/tokens/colors.test.ts
  - remote-frontend/src/styles/tokens/typography.test.ts
  - remote-frontend/.prettierignore
irreversible: false
scope_test: "remote-frontend/src/styles/tokens"
allowed_change: mixed
covers_criteria: [SC2]
---

## Failing test (write first)

Create `remote-frontend/src/styles/tokens/colors.test.ts`:

```ts
// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const colors = readFileSync(join(__dirname, 'colors.css'), 'utf-8');

describe('color tokens (SC2)', () => {
  it('defines dark-first HSL triplets + hex aliases for the 11 vks primitives', () => {
    for (const token of [
      '--vks-void', '--vks-surface', '--vks-surface-bright',
      '--vks-cyan', '--vks-amber', '--vks-emerald', '--vks-coral',
      '--vks-violet', '--vks-text', '--vks-text-muted', '--vks-text-dim',
    ]) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('defines the 5 status colors', () => {
    for (const token of ['--status-todo', '--status-inprogress', '--status-inreview', '--status-done', '--status-cancelled']) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('defines semantic aliases', () => {
    for (const token of ['--background', '--foreground', '--surface-card', '--primary', '--border', '--border-strong', '--ring']) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('defines the console palette', () => {
    for (const token of ['--console-bg', '--console-fg', '--console-muted', '--console-success', '--console-error', '--console-accent']) {
      expect(colors).toContain(`${token}:`);
    }
  });

  it('declares a light-mode opt-in via [data-theme="light"], .theme-light', () => {
    expect(colors).toMatch(/\[data-theme=['"]light['"]\]\s*,\s*\.theme-light/);
  });
});
```

Create `remote-frontend/src/styles/tokens/typography.test.ts`:

```ts
// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const typography = readFileSync(join(__dirname, 'typography.css'), 'utf-8');

describe('typography tokens (SC2)', () => {
  it('defines the 5 font families', () => {
    for (const token of ['--font-ui', '--font-code', '--font-display', '--font-wordmark', '--font-prose']) {
      expect(typography).toContain(`${token}:`);
    }
  });

  it('defines the downshifted type scale (base=14px)', () => {
    for (const token of ['--text-xs', '--text-sm', '--text-base', '--text-lg', '--text-xl', '--text-2xl', '--text-3xl', '--text-4xl', '--text-5xl']) {
      expect(typography).toContain(`${token}:`);
    }
    expect(typography).toContain('--text-base: 0.875rem');
  });

  it('defines leading, weight, and tracking tokens', () => {
    for (const token of ['--leading-tight', '--leading-relaxed', '--weight-regular', '--weight-bold', '--tracking-tight', '--tracking-wider']) {
      expect(typography).toContain(`${token}:`);
    }
  });
});
```

## Change

### File: `remote-frontend/src/styles/tokens/colors.css` (CREATE)
Copy byte-for-byte from `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/colors.css` (156 lines). Do NOT reformat — the `remote-frontend/.prettierignore` already excludes `src/types/shared/types.ts`; extend it to also exclude `src/styles/tokens/*.css` so Prettier cannot drift the copy.

The file declares, under `:root`, the HSL triplets (`--vks-void-hsl` etc.) + hex aliases (`--vks-void: #1a1a33` etc.) + semantic aliases (`--background`, `--foreground`, `--surface-card`, `--surface-raised`, `--surface-overlay`, `--text-body/muted/dim/on-accent`, `--primary/--primary-foreground`, `--accent/--accent-foreground`, `--border`, `--border-strong: #3f3f55`, `--input`, `--ring`) + feedback tokens (`--success/-foreground`, `--warning/-foreground`, `--danger/-foreground`, `--info/-foreground`) + 5 status tokens + console palette (`--console-bg: #0a0a0f` etc.) + a dark re-assert (`[data-theme='dark'], .theme-dark { color-scheme: dark }`) + a light-mode opt-in block under `[data-theme='light'], .theme-light` (bg `#f5f6f9`, primary `#0091b5`, etc.).

### File: `remote-frontend/src/styles/tokens/typography.css` (CREATE)
Copy byte-for-byte from `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/typography.css` (49 lines). Declares `--font-ui` (Inter+Noto Emoji), `--font-code` (JetBrains Mono+Chivo Mono), `--font-display` (Source Serif 4), `--font-wordmark` (Chivo Mono), `--font-prose` (Inter); the downshifted scale `--text-xs..--text-5xl` (base `--text-base: 0.875rem` = 14px); leading (`--leading-xs/sm/base/lg/xl` + `--leading-tight/snug/relaxed`); weights (`--weight-regular..bold`); tracking (`--tracking-tight/normal/wide/wider`).

### File: `remote-frontend/.prettierignore` (EDIT — extend the existing file from task 201)
- **Anchor:** the single line `src/types/shared/types.ts`
- **Before:** `src/types/shared/types.ts`
- **After:**
  ```
  src/types/shared/types.ts
  src/styles/tokens/*.css
  src/styles/components.css
  ```

**Note:** `remote-frontend/package.json` (vite `^8.0.7`, `@types/node`) and `remote-frontend/tsconfig.json` (`"node"` in types) were already updated in commit `4ca9ead9` to fix the vitest+vite incompatibility. Do NOT touch them — they are NOT in this task's `files:` list.

## Allowed moves

- Create `remote-frontend/src/styles/tokens/colors.css` as a byte-for-byte copy of the design-source file (use `cp`, not paste-into-editor).
- Create `remote-frontend/src/styles/tokens/typography.css` as a byte-for-byte copy.
- Create the two `.test.ts` files exactly as written above (with `// @vitest-environment node` directive).
- Append the three new lines to `remote-frontend/.prettierignore`.
- Do NOT edit `package.json` or `tsconfig.json` — they were already fixed in commit `4ca9ead9`.
- Run `cp` from the worktree root: `cp dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/colors.css remote-frontend/src/styles/tokens/colors.css` (and the same for typography.css).
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- The design-source file is missing or differs from the version recorded in the spec (the design-source tree is committed and immutable; if `git status` shows changes under `dev-docs/designs/.../design-source/`, STOP).
- `cp` reports a difference (run `diff` to confirm byte-identity after copy; any non-empty diff = STOP).
- The `.prettierignore` edit would clobber existing entries (the file already contains `src/types/shared/types.ts` from task 201 — match on that line, do not rewrite the file).
- A token the test asserts is absent from the source file (the source is authoritative; a missing token is a design-source defect → escalate, do not invent).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/tokens" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 101` exits 0. The test files use `// @vitest-environment node` + `readFileSync` (NOT `import.meta.glob` which is broken in vitest 4.1.3+vite 8). The `package.json` (vite `^8.0.7`, `@types/node`) and `tsconfig.json` (`"node"` in types) are already applied to the main tree (commit `4ca9ead9`) — do NOT touch them.