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
      body: JSON.stringify({
        authorize_url:
          'http://localhost:3002/oauth/callback?handoff_id=abc&app_code=xyz',
      }),
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

export async function setupTaskApiMocks(page: Page) {
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

/**
 * Minimal hive `SharedTask` wire shape consumed by BoardPage
 * (`src/lib/api/tasks.ts::Task`). `status` uses the kebab-case wire values
 * (`todo` / `in-progress` / `in-review` / `done` / `cancelled`).
 */
export interface MockTask {
  id: string;
  title: string;
  status: string;
  description?: string | null;
  owner_name?: string | null;
  executing_node_id?: string | null;
  owner_node_id?: string | null;
}

/**
 * Mock the BoardPage REST chain: organizations -> swarm projects -> bulk tasks
 * (`GET /v1/organizations`, `GET /v1/swarm/projects?...`, `GET /v1/tasks/bulk?...`).
 */
export async function setupBoardApiMocks(page: Page, tasks: MockTask[]) {
  await page.route('**/v1/organizations', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ organizations: [{ id: 'org1', name: 'Test Org' }] }),
    });
  });

  await page.route('**/v1/swarm/projects*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ projects: [{ id: 'p1', name: 'Test Project' }] }),
    });
  });

  await page.route('**/v1/tasks/bulk*', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        tasks: tasks.map((t) => ({
          task: {
            organization_id: 'org1',
            project_id: null,
            swarm_project_id: 'p1',
            creator_user_id: null,
            assignee_user_id: null,
            executing_node_id: null,
            owner_node_id: null,
            owner_name: null,
            description: null,
            version: 1,
            deleted_at: null,
            shared_at: null,
            archived_at: null,
            created_at: '2026-01-01T00:00:00Z',
            updated_at: '2026-01-01T00:00:00Z',
            ...t,
          },
          user: null,
        })),
        deleted_task_ids: [],
        latest_seq: null,
      }),
    });
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
