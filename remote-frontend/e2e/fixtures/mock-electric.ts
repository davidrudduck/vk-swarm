import type { Page } from '@playwright/test';

export interface MockTaskAssignment {
  id: string;
  task_id: string;
  node_id: string;
  node_project_id: string;
  execution_status: 'pending' | 'in_progress' | 'completed' | 'failed';
}

export type TableData = Record<string, unknown[]>;

export async function mockElectricShape(page: Page, tableData: TableData) {
  await page.route('**/v1/shape/*', async (route) => {
    const url = new URL(route.request().url());
    const segments = url.pathname.split('/');
    const tableName = segments[segments.length - 1];
    const result = tableData[tableName] ?? [];
    await route.fulfill({
      status: 200,
      contentType: 'application/x-ndjson',
      body: result.map((r) => JSON.stringify(r)).join('\n'),
    });
  });
}