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
    await expect(
      page.locator('button', { hasText: 'Sign in with GitHub' }),
    ).toBeVisible();
    await expect(
      page.locator('button', { hasText: 'Sign in with Google' }),
    ).toBeVisible();
  });

  test('OAuth init sends app_challenge in POST body', async ({ page }) => {
    await setupApiMocks(page);
    let initBody: string | null = null;

    await page.route('**/v1/oauth/web/init', async (route, request) => {
      initBody = request.postData();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          authorize_url: '/oauth/callback?handoff_id=abc&app_code=xyz',
        }),
      });
    });

    await page.goto('/login');
    await page
      .locator('button', { hasText: 'Sign in with GitHub' })
      .click();

    expect(initBody).not.toBeNull();
    const parsed = JSON.parse(initBody!);
    expect(parsed.app_challenge).toBeDefined();
    expect(parsed.app_challenge).toBeTruthy();
  });

  test('OAuth callback redeems handoff and stores access_token', async ({ page }) => {
    await setupApiMocks(page);
    // Per D-L10: seed the PKCE verifier so OAuthCallbackPage can redeem.
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');

    await page.waitForURL('**/nodes');
    const token = await page.evaluate(() =>
      localStorage.getItem('access_token'),
    );
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

    await page.waitForURL('**/login');
    const token = await page.evaluate(() =>
      localStorage.getItem('access_token'),
    );
    expect(token).toBeNull();
  });
});
