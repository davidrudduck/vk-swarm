import { test, expect } from '@playwright/test';
import { setupApiMocks, setupNodesApiMocks } from './fixtures/mock-api';
import { mockElectricShape } from './fixtures/mock-electric';
import type { MockTaskAssignment } from './fixtures/mock-electric';

const CROSS_NODE_ASSIGNMENTS: MockTaskAssignment[] = [
  { id: 'a1', task_id: 't-n1-1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
  { id: 'a2', task_id: 't-n2-1', node_id: 'n2', node_project_id: 'p2', execution_status: 'pending' },
  { id: 'a3', task_id: 't-n1-2', node_id: 'n1', node_project_id: 'p1', execution_status: 'in_progress' },
  { id: 'a4', task_id: 't-n2-2', node_id: 'n2', node_project_id: 'p2', execution_status: 'completed' },
];

test.describe('cross-node correctness (SC14)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
    mockElectricShape(page, CROSS_NODE_ASSIGNMENTS);
    await setupNodesApiMocks(page, [
      { id: 'n1', name: 'node-alpha' },
      { id: 'n2', name: 'node-beta' },
    ]);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');
    await page.waitForURL('**/nodes');
    await page.goto('/tasks');
  });

  test('tasks from two different nodes appear in same pending column', async ({ page }) => {
    await expect(page.locator('text=task t-n1-1')).toBeVisible();
    await expect(page.locator('text=task t-n2-1')).toBeVisible();
  });

  test('TaskDetail shows correct node_id label per task', async ({ page }) => {
    await page.locator('li').first().click();
    await expect(page.locator('text=node-alpha')).toBeVisible();

    await page.locator('li').nth(1).click();
    await expect(page.locator('text=node-beta')).toBeVisible();
  });

  test('tasks from both nodes span all status columns', async ({ page }) => {
    await expect(page.locator('h2:has-text("pending")')).toBeVisible();
    await expect(page.locator('h2:has-text("in progress")')).toBeVisible();
    await expect(page.locator('h2:has-text("completed")')).toBeVisible();

    await expect(page.locator('text=task t-n1-1')).toBeVisible();
    await expect(page.locator('text=task t-n1-2')).toBeVisible();
    await expect(page.locator('text=task t-n2-1')).toBeVisible();
    await expect(page.locator('text=task t-n2-2')).toBeVisible();
  });
});
