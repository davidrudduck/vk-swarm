import type { Page } from '@playwright/test';

export interface MockTaskAssignment {
  id: string;
  task_id: string;
  node_id: string;
  node_project_id: string;
  execution_status: 'pending' | 'in_progress' | 'completed' | 'failed';
}

export interface MockNode {
  id: string;
  name: string;
  organization_id?: string;
  hostname?: string | null;
  status?: string;
  last_heartbeat_at?: string | null;
  public_url?: string | null;
  created_at?: string;
  updated_at?: string;
}

export type TableData = Record<string, unknown[]>;

export function mockElectricShape(page: Page, tableData: TableData) {
  page.route('**/api/electric/v1/shape/*', async (route) => {
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