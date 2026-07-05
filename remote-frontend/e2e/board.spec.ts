import { test, expect } from '@playwright/test';
import {
  setupApiMocks,
  setupTaskApiMocks,
  setupNodesApiMocks,
} from './fixtures/mock-api';
import { mockElectricShape } from './fixtures/mock-electric';
import type { MockTaskAssignment } from './fixtures/mock-electric';

const MOCK_ASSIGNMENTS: MockTaskAssignment[] = [
  { id: 'a1', task_id: 't1', node_id: 'n1', node_project_id: 'p1', execution_status: 'pending' },
  { id: 'a2', task_id: 't2', node_id: 'n1', node_project_id: 'p1', execution_status: 'in_progress' },
  { id: 'a3', task_id: 't3', node_id: 'n2', node_project_id: 'p2', execution_status: 'completed' },
  { id: 'a4', task_id: 't4', node_id: 'n2', node_project_id: 'p2', execution_status: 'failed' },
];

test.describe('kanban board (SC13)', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
    mockElectricShape(page, MOCK_ASSIGNMENTS);
    await setupNodesApiMocks(page, [
      { id: 'n1', name: 'node-alpha' },
      { id: 'n2', name: 'node-beta' },
    ]);
    await page.addInitScript(() => {
      sessionStorage.setItem('oauth_verifier', 'test-verifier');
    });
    await page.goto('/oauth/callback?handoff_id=abc&app_code=xyz');
    await page.waitForURL('**/nodes');
  });

  test('4 columns visible with correct headers', async ({ page }) => {
    await page.goto('/tasks');
    await expect(page.locator('h2')).toContainText([
      'pending',
      'in progress',
      'completed',
      'failed',
    ]);
  });

  test('tasks from mock data appear in correct columns', async ({ page }) => {
    await page.goto('/tasks');
    await expect(page.locator('text=task t1')).toBeVisible();
    await expect(page.locator('text=task t2')).toBeVisible();
    await expect(page.locator('text=task t3')).toBeVisible();
    await expect(page.locator('text=task t4')).toBeVisible();
  });

  test('card click opens TaskDetail panel', async ({ page }) => {
    await page.goto('/tasks');
    await page.locator('li').first().click();
    await expect(page.locator('text=Progress events')).toBeVisible();
  });

  test('assign fires PATCH with correct body', async ({ page }) => {
    let patchBody: string | null = null;
    await page.route('**/v1/tasks/*/executing-node', async (route, request) => {
      if (request.method() === 'PATCH') {
        patchBody = request.postData();
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: '{"ok":true}',
        });
      } else {
        await route.fulfill({ status: 500 });
      }
    });

    await page.goto('/tasks');
    await page.locator('select').first().selectOption('n1');
    await page.locator('[aria-label="Assign"]').first().click();

    expect(patchBody).not.toBeNull();
    const parsed = JSON.parse(patchBody!);
    expect(parsed).toHaveProperty('node_id');
  });

  test('delete shows confirmation dialog then fires DELETE', async ({ page }) => {
    let deleteCalled = false;
    await page.route('**/v1/tasks/*', async (route, request) => {
      if (request.method() === 'DELETE') {
        deleteCalled = true;
        await route.fulfill({ status: 204 });
      } else if (request.method() === 'PATCH') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: '{"ok":true}',
        });
      } else {
        await route.fulfill({ status: 500 });
      }
    });

    await page.goto('/tasks');
    await page.locator('[aria-label="Delete"]').first().click();
    await expect(page.locator('text=Are you sure?')).toBeVisible();
    await page.locator('button', { hasText: 'Delete' }).last().click();

    expect(deleteCalled).toBe(true);
  });

  test('DELETE 500 shows error toast and card stays', async ({ page }) => {
    await page.route('**/v1/tasks/*', async (route, request) => {
      if (request.method() === 'DELETE') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: '{"error":"server error"}',
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: '{"ok":true}',
        });
      }
    });

    await page.goto('/tasks');
    await page.locator('[aria-label="Delete"]').first().click();
    await expect(page.locator('text=Are you sure?')).toBeVisible();
    await page.locator('button', { hasText: 'Delete' }).last().click();

    await expect(page.locator('text=Delete failed')).toBeVisible({
      timeout: 5000,
    });
    await expect(page.locator('text=task t1')).toBeVisible();
  });
});
