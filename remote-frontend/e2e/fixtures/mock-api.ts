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

export async function setupNodesApiMocks(page: Page, nodes: unknown[]) {
  await page.route('**/v1/nodes', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(nodes),
    });
  });
}
