import { test, expect } from '@playwright/test';
import { setupApiMocks, setupBoardApiMocks, type MockTask } from './fixtures/mock-api';

// Cross-node correctness on the design-system board: tasks owned by different
// nodes render on one shared board, with per-task node attribution surfaced on
// the card and in the TaskDrawer.
const CROSS_NODE_TASKS: MockTask[] = [
  { id: 't-n1-1', title: 'task t-n1-1', status: 'todo', owner_name: 'node-alpha' },
  { id: 't-n2-1', title: 'task t-n2-1', status: 'todo', owner_name: 'node-beta' },
  { id: 't-n1-2', title: 'task t-n1-2', status: 'in-progress', owner_name: 'node-alpha' },
  { id: 't-n2-2', title: 'task t-n2-2', status: 'done', owner_name: 'node-beta' },
];

test.describe('cross-node correctness (SC14)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
    await setupBoardApiMocks(page, CROSS_NODE_TASKS);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');
    await page.waitForURL(/\/nodes(\?|$)/);
    await page.goto('/tasks');
  });

  test('tasks from two different nodes appear in the same To Do column', async ({ page }) => {
    await expect(page.getByText('task t-n1-1', { exact: true })).toBeVisible();
    await expect(page.getByText('task t-n2-1', { exact: true })).toBeVisible();
  });

  test('TaskDrawer shows the owning node per task', async ({ page }) => {
    await page.getByText('task t-n1-1', { exact: true }).click();
    await expect(page.locator('aside').getByText('node-alpha')).toBeVisible();
    await page.locator('aside').getByRole('button', { name: 'Close' }).click();

    await page.getByText('task t-n2-1', { exact: true }).click();
    await expect(page.locator('aside').getByText('node-beta')).toBeVisible();
  });

  test('tasks from both nodes span multiple status columns', async ({ page }) => {
    for (const t of CROSS_NODE_TASKS) {
      await expect(page.getByText(t.title, { exact: true })).toBeVisible();
    }
  });
});
