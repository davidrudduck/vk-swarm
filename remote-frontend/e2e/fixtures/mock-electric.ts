import type { Page } from '@playwright/test';

export interface MockTaskAssignment {
  id: string;
  task_id: string;
  node_id: string;
  node_project_id: string;
  execution_status: 'pending' | 'in_progress' | 'completed' | 'failed';
}

export function mockElectricShape(page: Page, assignments: MockTaskAssignment[]) {
  page.route('**/api/electric/v1/shape/*', async (route) => {
    const url = new URL(route.request().url());
    const segments = url.pathname.split('/');
    const tableName = segments[segments.length - 1];
    const result = tableName === 'node_task_assignments' ? assignments : [];
    await route.fulfill({
      status: 200,
      contentType: 'application/x-ndjson',
      body: result.map((r) => JSON.stringify(r)).join('\n'),
    });
  });
}
