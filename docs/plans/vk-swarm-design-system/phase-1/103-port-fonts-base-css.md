---
id: "103"
phase: 1
title: Port fonts @import + base element CSS into remote-frontend
status: ready
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/styles/tokens/fonts.css
  - remote-frontend/src/styles/tokens/base.css
  - remote-frontend/src/styles/tokens/base.test.ts
irreversible: false
scope_test: "remote-frontend/src/styles/tokens/base.test.ts"
allowed_change: create
covers_criteria: [SC2]
---

## Failing test (write first)

Create `remote-frontend/src/styles/tokens/base.test.ts`:

```ts
// @vitest-environment node
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fonts = readFileSync(join(__dirname, 'fonts.css'), 'utf-8');
const base = readFileSync(join(__dirname, 'base.css'), 'utf-8');

describe('fonts (SC2)', () => {
  it('imports the 5 font families from Google Fonts', () => {
    expect(fonts).toContain('fonts.googleapis.com');
    for (const family of ['Inter', 'JetBrains+Mono', 'Chivo+Mono', 'Noto+Emoji', 'Source+Serif+4']) {
      expect(fonts).toContain(family);
    }
  });
});

describe('base element CSS (SC2)', () => {
  it('sets color-scheme dark by default', () => {
    expect(base).toMatch(/html\s*\{[^}]*color-scheme:\s*dark/);
  });

  it('resets body margin and applies background/foreground/font', () => {
    expect(base).toContain('body');
    expect(base).toContain('margin: 0');
    expect(base).toContain('var(--background)');
    expect(base).toContain('var(--foreground)');
  });

  it('styles anchor, code, selection', () => {
    expect(base).toContain('::selection');
    expect(base).toContain('code');
    expect(base).toContain('a ');
  });
});
```

## Change

### File: `remote-frontend/src/styles/tokens/fonts.css` (CREATE)
Copy byte-for-byte from `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/fonts.css` (7 lines) via `cp`. A single `@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&family=Chivo+Mono:wght@300;400;500;600;700&family=Noto+Emoji:wght@300..700&family=Source+Serif+4:opsz,wght@8..60,300;8..60,400;8..60,600;8..60,700&display=swap')`.

### File: `remote-frontend/src/styles/tokens/base.css` (CREATE)
Copy byte-for-byte from `dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/base.css` (134 lines) via `cp`. Sets `box-sizing`, `html { color-scheme: dark }`, body (margin 0, bg/foreground/font/size/leading/font-smoothing/text-rendering), `::selection` (cyan 0.3 bg), `a` (primary color, no decoration, hover underline), `code/pre/kbd/samp` (font-code).

NOTE: `base.css` ALSO contains the texture utilities (`.vks-diagonal-lines`, `.vks-ansi-dither`, `.vks-ansi-weave`, `.vks-ansi-grid`, `.vks-scanlines`, `.vks-dashed`, `.vks-wordmark`, `.vks-eyebrow`). Task 104 will add a dedicated DOM test for those; this task's test only asserts the base element CSS. The file is copied whole here because the design source ships it as one file; task 104 only adds tests, it does NOT re-copy.

## Allowed moves

- `cp dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/fonts.css remote-frontend/src/styles/tokens/fonts.css`
- `cp dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/tokens/base.css remote-frontend/src/styles/tokens/base.css`
- Create the `.test.ts` file exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `diff` is non-empty after `cp` for either file (byte-identity required).
- `base.css` does not contain the texture utilities (would mean wrong file copied → STOP and re-copy).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/tokens/base.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 103` exits 0.