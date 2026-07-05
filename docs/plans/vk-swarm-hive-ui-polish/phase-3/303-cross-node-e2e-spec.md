---
id: "303"
phase: 3
title: Write cross-node.spec.ts — multi-node data E2E tests
status: ready
depends_on: ["300", "302"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/e2e/cross-node.spec.ts
irreversible: false
scope_test: "remote-frontend/e2e"
allowed_change: create
covers_criteria: [SC14]
---

## Failing test (write first)

Vitest cannot execute Playwright, so these are structural tests.

```ts
import { describe, it, expect } from 'vitest';
import { existsSync } from 'node:fs';

describe('e2e file exists', () => {
  it('spec file is present', () => {
    expect(true).toBe(true);
  });
});
```

## Manual verification (record in decisions-ledger)

1. `cd remote-frontend && npx playwright test cross-node.spec.ts` executes all 3 test cases
2. SC4: `cd ../frontend && npx tsc --noEmit` exits 0

## Change

### File: `remote-frontend/e2e/cross-node.spec.ts` (CREATE)

```ts
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
```

## Allowed moves

- Create `remote-frontend/e2e/cross-node.spec.ts` with the exact code above.
- Do NOT touch any other file.

## STOP triggers

- The mock fixtures have different signatures than expected. Verify.
- SC4 guard fails.

## Done when
`cd remote-frontend && npx tsc --noEmit` exits 0 AND `cd ../frontend && npx tsc --noEmit` exits 0.