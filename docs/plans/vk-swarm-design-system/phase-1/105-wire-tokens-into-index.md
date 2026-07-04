---
id: "105"
phase: 1
title: Wire tokens into remote-frontend index.css entry
status: ready
depends_on: ["101","102","103","104"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/index.css
  - remote-frontend/src/styles/tokens/index.test.ts
irreversible: false
scope_test: "remote-frontend/src/styles/tokens/index.test.ts"
allowed_change: mixed
covers_criteria: [SC2, SC3]
---

## Failing test (write first)

Create `remote-frontend/src/styles/tokens/index.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const indexCss = readFileSync(join(__dirname, '..', '..', 'index.css'), 'utf8');

describe('index.css wires the token files (SC2/SC3)', () => {
  it('@imports fonts.css before colors/typography/spacing/base', () => {
    expect(indexCss).toContain("@import './styles/tokens/fonts.css';");
    expect(indexCss).toContain("@import './styles/tokens/colors.css';");
    expect(indexCss).toContain("@import './styles/tokens/typography.css';");
    expect(indexCss).toContain("@import './styles/tokens/spacing.css';");
    expect(indexCss).toContain("@import './styles/tokens/base.css';");
  });

  it('keeps the existing tailwind directives', () => {
    expect(indexCss).toContain('@tailwind base');
    expect(indexCss).toContain('@tailwind components');
    expect(indexCss).toContain('@tailwind utilities');
  });

  it('places @import statements before @tailwind directives (CSS @import ordering rule)', () => {
    const firstImport = indexCss.indexOf('@import');
    const firstTailwind = indexCss.indexOf('@tailwind');
    expect(firstImport).toBeLessThan(firstTailwind);
  });
});
```

## Change

### File: `remote-frontend/src/index.css` (EDIT)
- **Anchor:** the 3-line file (the whole file).
- **Before:**
  ```css
  @tailwind base;
  @tailwind components;
  @tailwind utilities;
  ```
- **After:**
  ```css
  @import './styles/tokens/fonts.css';
  @import './styles/tokens/colors.css';
  @import './styles/tokens/typography.css';
  @import './styles/tokens/spacing.css';
  @import './styles/tokens/base.css';

  @tailwind base;
  @tailwind components;
  @tailwind utilities;
  ```

CSS requires `@import` statements to precede all other rules; placing them before the `@tailwind` directives satisfies that. The font import is first so the Google Fonts request fires before any token that references the families.

## Allowed moves

- Edit `remote-frontend/src/index.css` to prepend exactly the 5 `@import` lines shown above (in that order), keeping the 3 existing `@tailwind` lines.
- Create the `.test.ts` file exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `remote-frontend/src/index.css` is not the 3-line file shown in Before (would mean an earlier task drifted → STOP, escalate).
- The `@import` paths do not resolve (run `cd remote-frontend && npx vite build` after the edit; a failed resolution fails the build → STOP).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/tokens/index.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 105` exits 0.