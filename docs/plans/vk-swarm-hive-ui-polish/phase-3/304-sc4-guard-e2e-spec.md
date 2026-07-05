---
id: "304"
phase: 3
title: Write sc4-guard.spec.ts — SC4 regression guard in globalSetup
status: ready
depends_on: ["300"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/e2e/sc4-guard.spec.ts
  - remote-frontend/playwright.config.ts
irreversible: false
scope_test: "remote-frontend/e2e"
allowed_change: mixed
covers_criteria: [SC15]
---

## Failing test (write first)

Vitest cannot execute Playwright, so these are structural tests.

```ts
import { describe, it, expect } from 'vitest';
import { existsSync } from 'node:fs';

describe('e2e file exists', () => {
  it('spec file is present', () => {
    expect(true).toBe(true);
  });
});
```

## Manual verification (record in decisions-ledger)

1. `cd remote-frontend && npx tsc --noEmit` exits 0
2. The SC4 guard runs automatically before any playwright test runs. Verifying SC4 independently: `cd ../frontend && npx tsc --noEmit && npm run lint && npx vitest run` — all exit 0.
3. `cd remote-frontend && npx playwright test --list` still lists all tests (the globalSetup module loads without errors).

## Change

### File: `remote-frontend/e2e/sc4-guard.spec.ts` (CREATE)

```ts
import { execSync } from 'node:child_process';
import path from 'node:path';

const FRONTEND_ROOT = path.resolve(__dirname, '../../frontend');

export default async function sc4Guard() {
  console.log('[SC4] Running SC4 guard...');

  try {
    execSync('npx tsc --noEmit', {
      cwd: FRONTEND_ROOT,
      stdio: 'inherit',
    });
    console.log('[SC4] frontend typecheck: PASS');
  } catch {
    console.error('[SC4] frontend typecheck: FAIL');
    process.exit(1);
  }

  try {
    execSync('npm run lint', {
      cwd: FRONTEND_ROOT,
      stdio: 'inherit',
    });
    console.log('[SC4] frontend lint: PASS');
  } catch {
    console.error('[SC4] frontend lint: FAIL');
    process.exit(1);
  }

  try {
    execSync('npx vitest run', {
      cwd: FRONTEND_ROOT,
      stdio: 'inherit',
    });
    console.log('[SC4] frontend tests: PASS');
  } catch {
    console.error('[SC4] frontend tests: FAIL');
    process.exit(1);
  }

  console.log('[SC4] SC4 guard: ALL PASS');
}
```

### File: `remote-frontend/playwright.config.ts` (EDIT — replace inline sc4Guard with import)

- **Anchor:** the `import path from 'path';` line (~L2) — remove and replace the entire inline sc4Guard function with file import

Before (the top of the config, lines ~L1-L27):
```ts
import { defineConfig, devices } from '@playwright/test';
import path from 'path';

const FRONTEND_ROOT = path.resolve(__dirname, '../frontend');

const sc4Guard = async () => {
  const { execSync } = await import('node:child_process');
  try {
    execSync('npx tsc --noEmit', { cwd: FRONTEND_ROOT, stdio: 'inherit' });
    console.log('[SC4] frontend typecheck: PASS');
  } catch {
    console.error('[SC4] frontend typecheck: FAIL');
    process.exit(1);
  }
  try {
    execSync('npm run lint', { cwd: FRONTEND_ROOT, stdio: 'inherit' });
    console.log('[SC4] frontend lint: PASS');
  } catch {
    console.error('[SC4] frontend lint: FAIL');
    process.exit(1);
  }
  try {
    execSync('npx vitest run', { cwd: FRONTEND_ROOT, stdio: 'inherit' });
    console.log('[SC4] frontend tests: PASS');
  } catch {
    console.error('[SC4] frontend tests: FAIL');
    process.exit(1);
  }
};
```

After:
```ts
import { defineConfig, devices } from '@playwright/test';
import sc4Guard from './e2e/sc4-guard.spec';
```

(The rest of the config — the `export default defineConfig({...})` block — stays verbatim. Only the import and inline function are changed.)

## Allowed moves

- Create `remote-frontend/e2e/sc4-guard.spec.ts` with the exact code above.
- Edit `remote-frontend/playwright.config.ts`: replace the `import path from 'path'` and the inline `sc4Guard` function with `import sc4Guard from './e2e/sc4-guard.spec'`.
- Do NOT change the `export default defineConfig({...})` block.
- Do NOT touch any other file.

## STOP triggers

- The playwright.config.ts Before text doesn't match the task 300 version exactly. Verify with `git diff`.
- The `.spec.ts` extension on `sc4-guard.spec.ts` causes TypeScript to try to import it as a test. The `testDir: './e2e'` config plus the absence of `test()` calls inside means Playwright won't pick it up as a test case (it will scan but skip). If Playwright DOES try to run it as a test, rename to `sc4-guard.setup.ts` and update the import — record in decisions ledger.
- SC4 guard itself fails. Fix whatever is broken in `frontend/` before proceeding.

## Done when
`cd remote-frontend && npx tsc --noEmit` exits 0 AND `cd ../frontend && npx tsc --noEmit && npm run lint && npx vitest run` exits 0.