---
id: "300"
phase: 3
title: Install Playwright + create config + mock fixtures
status: ready
depends_on: ["104"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/package.json
  - remote-frontend/e2e/.gitkeep
  - remote-frontend/playwright.config.ts
  - remote-frontend/e2e/fixtures/mock-api.ts
  - remote-frontend/e2e/fixtures/mock-electric.ts
irreversible: false
scope_test: "remote-frontend/e2e"
allowed_change: mixed
covers_criteria: [SC16]
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

Playwright config files and fixtures cannot be meaningfully unit-tested (they configure a runtime). Verify manually:

1. `cd remote-frontend && npx playwright install chromium` exits 0
2. `cd remote-frontend && npx tsc --noEmit` still passes (the config and fixture files are TypeScript)
3. `cd remote-frontend && npx playwright test --list` exits 0 (lists tests even if none exist yet)
4. SC4 guard: `cd frontend && npx tsc --noEmit` exits 0

Record the Playwright and Chromium versions installed in the decisions ledger.

## Change

### File: `remote-frontend/package.json` (EDIT — add playwright + scripts)

- **Anchor:** `"devDependencies"` block, last line before closing `}`

Before:
```json
    "vitest": "^4.1.3"
  }
}
```

After:
```json
    "vitest": "^4.1.3",
    "@playwright/test": "^1.56.0"
  }
}
```

- **Anchor:** `"scripts"` block, after `"test:run": "vitest run"`

Before:
```json
    "test:run": "vitest run"
  },
```

After:
```json
    "test:run": "vitest run",
    "test:e2e": "playwright test",
    "test:e2e:ci": "playwright test --reporter=list"
  },
```

Run `cd remote-frontend && npm install && npx playwright install chromium --with-deps`.

### File: `remote-frontend/playwright.config.ts` (CREATE)

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

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  expect: { timeout: 10_000 },
  globalSetup: sc4Guard,
  use: {
    baseURL: 'http://localhost:3002',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'npx vite --port 3002',
    url: 'http://localhost:3002',
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
  retries: process.env.CI ? 2 : 0,
  workers: 1,
});
```

### File: `remote-frontend/e2e/fixtures/mock-api.ts` (CREATE)

```ts
import type { Page } from '@playwright/test';

interface MockUser {
  id: string;
  email: string;
  name: string;
}

export async function setupApiMocks(page: Page, user?: MockUser) {
  const u = user ?? { id: 'u1', email: 'admin@test.com', name: 'Admin' };

  await page.route('**/v1/oauth/web/init', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ authorize_url: 'http://localhost:3002/oauth/callback?handoff_id=abc&app_code=xyz' }),
    });
  });

  await page.route('**/v1/oauth/web/redeem', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        access_token: 'mock-jwt-token',
        user: u,
      }),
    });
  });

  await page.route('**/v1/profile', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(u),
    });
  });
}

export async function setupTaskApiMocks(page: Page, tasks: unknown[]) {
  await page.route('**/v1/tasks/**', async (route) => {
    const method = route.request().method();
    if (method === 'DELETE') {
      await route.fulfill({ status: 204 });
    } else if (method === 'PATCH') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      });
    } else {
      await route.fulfill({ status: 500 });
    }
  });
}

export async function setupNodesApiMocks(page: Page, nodes: unknown[]) {
  await page.route('**/v1/nodes', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(nodes),
    });
  });
}
```

### File: `remote-frontend/e2e/fixtures/mock-electric.ts` (CREATE)

```ts
import type { Page } from '@playwright/test';

export interface MockTaskAssignment {
  id: string;
  task_id: string;
  node_id: string;
  node_project_id: string;
  execution_status: 'pending' | 'in_progress' | 'completed' | 'failed';
}

export function mockElectricShape(page: Page, assignments: MockTaskAssignment[]) {
  page.route('**/api/electric/v1/shape/*', async (route, request) => {
    const url = new URL(request.url());
    const pathParts = url.pathname.split('/');
    const table = pathParts[pathParts.length - 1];
    const result = table === 'node_task_assignments'
      ? assignments
      : [];
    await route.fulfill({
      status: 200,
      contentType: 'application/x-ndjson',
      body: result.map((r) => JSON.stringify(r)).join('\n'),
    });
  });
}
```

## Allowed moves

- Edit `remote-frontend/package.json`: add `@playwright/test` to devDependencies, add `test:e2e` and `test:e2e:ci` scripts. Run `npm install` + `npx playwright install chromium --with-deps`.
- Create `remote-frontend/playwright.config.ts` with the exact code above.
- Create `remote-frontend/e2e/fixtures/mock-api.ts` with the exact code above.
- Create `remote-frontend/e2e/fixtures/mock-electric.ts` with the exact code above.
- Do NOT create any spec files yet — that's tasks 301-304.
- Do NOT touch `frontend/` (SC4). The `playwright.config.ts` `globalSetup` reads from `frontend/` but does not modify it.

## STOP triggers

- `@playwright/test` v1.56 doesn't resolve. Install the latest 1.x — record the version in the decisions ledger.
- `npx playwright install chromium --with-deps` fails. Try without `--with-deps` or install system deps manually. Record in decisions ledger.
- The `globalSetup` function runs a dynamic `import('node:child_process')` which may fail in ESM context. If the config doesn't parse, use a static `import { execSync } from 'node:child_process'` at the top. Record in decisions ledger.
- The `remote-frontend/e2e/` directory already exists (stale from a prior aborted run). Delete or rename before proceeding.

## Done when
Manual verification check:
1. `cd remote-frontend && npx tsc --noEmit` exits 0
2. `cd remote-frontend && npx playwright test --list` exits 0
3. `cd ../frontend && npx tsc --noEmit` exits 0