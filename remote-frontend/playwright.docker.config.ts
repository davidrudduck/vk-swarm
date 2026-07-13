import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright config for E2E testing against the Docker environment.
 * Uses the real server at http://localhost:9000 (no mocks).
 *
 * Usage:
 *   npx playwright test --config=playwright.docker.config.ts
 *   ./scripts/e2e-test.sh  (orchestrates Docker + this config)
 */
export default defineConfig({
  testDir: './e2e',
  testIgnore: ['**/sc4-guard.spec.ts'],
  timeout: 30_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:9000',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  // No webServer — Docker is already running
  retries: 0,
  workers: 1,
});
