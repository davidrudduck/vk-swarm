import { test, expect } from '@playwright/test';
import { setupApiMocks, setupBoardApiMocks, type MockTask } from './fixtures/mock-api';

// Board rendered by BoardView (design-system app shell): five columns fed by
// the REST chain organizations -> swarm projects -> /v1/tasks/bulk. Statuses
// are the kebab-case hive wire values normalized in BoardPage.
const MOCK_TASKS: MockTask[] = [
  { id: 't1', title: 'task t1', status: 'todo', owner_name: 'node-alpha' },
  { id: 't2', title: 'task t2', status: 'in-progress', owner_name: 'node-alpha' },
  { id: 't3', title: 'task t3', status: 'in-review', owner_name: 'node-beta' },
  { id: 't4', title: 'task t4', status: 'done', owner_name: 'node-beta' },
  { id: 't5', title: 'task t5', status: 'cancelled', owner_name: 'node-beta' },
];

test.describe('kanban board (SC13)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
    await setupBoardApiMocks(page, MOCK_TASKS);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');
    await page.waitForURL(/\/nodes(\?|$)/);
    await page.goto('/tasks');
  });

  test('5 columns visible with correct headers', async ({ page }) => {
    for (const label of ['To Do', 'In Progress', 'In Review', 'Done', 'Cancelled']) {
      await expect(page.getByText(label, { exact: true })).toBeVisible();
    }
  });

  test('tasks from mock data appear on the board', async ({ page }) => {
    for (const t of MOCK_TASKS) {
      await expect(page.getByText(t.title, { exact: true })).toBeVisible();
    }
  });

  test('card click opens TaskDrawer with title and node badge', async ({ page }) => {
    await page.getByText('task t1', { exact: true }).click();
    await expect(page.locator('aside').getByRole('heading', { name: 'task t1' })).toBeVisible();
    await expect(page.locator('aside').getByText('node-alpha')).toBeVisible();
  });

  test('TaskDrawer close button dismisses the drawer', async ({ page }) => {
    await page.getByText('task t2', { exact: true }).click();
    const drawer = page.locator('aside');
    await expect(drawer).toBeVisible();
    await drawer.getByRole('button', { name: 'Close' }).click();
    await expect(drawer).toHaveCount(0);
  });

  test('bulk-tasks failure shows error banner', async ({ page }) => {
    await page.route('**/v1/tasks/bulk*', async (route) => {
      await route.fulfill({ status: 500, body: 'server error' });
    });
    await page.reload();
    await expect(
      page.getByText('Failed to load tasks. Check your connection and try again.'),
    ).toBeVisible();
  });
});
