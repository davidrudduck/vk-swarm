import { test, expect } from '@playwright/test';

/**
 * Docker E2E tests — run against the real server at localhost:9000.
 * No mocks. Tests the actual API + UI integration.
 *
 * These tests assume the Docker environment is running with the E2E seed data.
 * Run: ./scripts/e2e-test.sh
 */

test.describe('Docker E2E — Health & Frontend', () => {
  test('health endpoint returns ok', async ({ request }) => {
    const response = await request.get('/v1/health');
    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.status).toBe('ok');
    expect(body.version).toBeDefined();
  });

  test('frontend serves index.html and redirects to login', async ({ page }) => {
    const response = await page.goto('/');
    expect(response?.status()).toBe(200);
    // Should redirect to /login (no auth token)
    await page.waitForURL('**/login');
    await expect(page.locator('h1')).toContainText('Welcome');
  });

  test('login page renders OAuth buttons', async ({ page }) => {
    await page.goto('/login');
    await expect(
      page.locator('button', { hasText: 'Sign in with GitHub' }),
    ).toBeVisible();
  });
});


test.describe('Docker E2E — Static Assets', () => {
  test('JS bundle loads', async ({ page }) => {
    await page.goto('/');
    // Wait for React to mount
    await page.waitForSelector('#root');
    // The login page should render
    await expect(page.locator('h1')).toContainText('Welcome');
  });

  test('CSS loads (dark theme applied)', async ({ page }) => {
    await page.goto('/login');
    // Verify that the root element has non-default styling (not browser default white)
    const bgColor = await page.evaluate(() =>
      getComputedStyle(document.getElementById('root')!).backgroundColor,
    );
    expect(bgColor).not.toBe('rgb(255, 255, 255)');
  });
});

test.describe('Docker E2E — Error Handling', () => {
  test('404 page renders for unknown routes', async ({ page }) => {
    const response = await page.goto('/nonexistent-page-12345');
    // Unknown routes should not return 200 OK
    expect(response?.ok()).toBeFalsy();
    await expect(page.locator('body')).not.toBeEmpty();
  });

  test('login page renders successfully across repeated navigations', async ({ page }) => {
    // Navigate to login, then away to root (which redirects to login),
    // then back to login to verify no crashes across transitions
    await page.goto('/login');
    await expect(page.locator('h1')).toContainText('Welcome');
    await page.goto('/nonexistent-route-999');
    await page.goto('/login');
    await expect(page.locator('h1')).toContainText('Welcome');
  });
});
