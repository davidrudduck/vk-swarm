---
id: "301"
phase: 3
title: Write auth.spec.ts — OAuth PKCE flow E2E tests
status: ready
depends_on: ["300"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/e2e/auth.spec.ts
irreversible: false
scope_test: "remote-frontend/e2e"
allowed_change: create
covers_criteria: [SC12]
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

E2E tests cannot be unit-tested by vitest. Verify manually:

1. `cd remote-frontend && npx playwright test auth.spec.ts` executes the 6 test cases described below
2. All tests pass (or at minimum the test file compiles and is discoverable by `npx playwright test --list`)
3. SC4: `cd ../frontend && npx tsc --noEmit` exits 0

Record the test run results in the decisions ledger.

## Change

### File: `remote-frontend/e2e/auth.spec.ts` (CREATE)

```ts
import { test, expect } from '@playwright/test';
import { setupApiMocks } from './fixtures/mock-api';

test.describe('OAuth PKCE auth flow (SC12)', () => {
  test('unauthenticated user at / is redirected to /login', async ({ page }) => {
    await page.goto('/');
    await page.waitForURL('**/login');
    await expect(page.locator('h1')).toContainText('Welcome');
  });

  test('login page shows GitHub and Google buttons', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('button', { hasText: 'Sign in with GitHub' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Sign in with Google' })).toBeVisible();
  });

  test('OAuth init sends app_challenge in POST body', async ({ page }) => {
    await setupApiMocks(page);
    let initBody: string | null = null;

    await page.route('**/v1/oauth/web/init', async (route, request) => {
      initBody = request.postData();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ authorize_url: '/oauth/callback?handoff_id=abc&app_code=xyz' }),
      });
    });

    await page.goto('/login');
    await page.locator('button', { hasText: 'Sign in with GitHub' }).click();

    expect(initBody).not.toBeNull();
    const parsed = JSON.parse(initBody!);
    expect(parsed.app_challenge).toBeDefined();
    expect(parsed.app_challenge).toBeTruthy();
  });

  test('OAuth callback redeems handoff and stores access_token', async ({ page }) => {
    await setupApiMocks(page);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');

    await page.waitForURL('**/nodes');
    const token = await page.evaluate(() => localStorage.getItem('access_token'));
    expect(token).toBe('mock-jwt-token');
  });

  test('post-login redirects to /nodes', async ({ page }) => {
    await setupApiMocks(page);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');
    await page.waitForURL('**/nodes');
    await expect(page).toHaveURL(/\/nodes/);
  });

  test('logout clears token and redirects to /login', async ({ page }) => {
    await setupApiMocks(page);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');
    await page.waitForURL('**/nodes');

    await page.route('**/v1/oauth/logout', async (route) => {
      await route.fulfill({ status: 204 });
    });

    await page.locator('[aria-label="Logout"]').click();

    const token = await page.evaluate(() => localStorage.getItem('access_token'));
    expect(token).toBeNull();
  });
});
```

## Allowed moves

- Create `remote-frontend/e2e/auth.spec.ts` with the exact code above.
- Do NOT touch any other file.

## STOP triggers

- The `setupApiMocks` function from task 300 doesn't exist or has a different signature. Verify with `ls remote-frontend/e2e/fixtures/mock-api.ts`.
- The Playwright config `baseURL` (port 3002) doesn't match the actual dev server port. If the dev server runs on a different port, update `playwright.config.ts`.
- The SC4 guard fails — verify `cd ../frontend && npx tsc --noEmit` before claiming the manual verification is complete.

## Done when
`cd remote-frontend && npx tsc --noEmit` exits 0 (typecheck) AND `cd ../frontend && npx tsc --noEmit` exits 0 (SC4). The E2E tests themselves are verified manually.