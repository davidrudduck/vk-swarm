---
id: "307"
phase: 3
title: Wire management actions (set executing node / delete) to hive REST routes
status: ready
depends_on: ["305"]
parallel: false
conflicts_with: ["306"]
files:
  - remote-frontend/src/pages/Tasks.tsx
  - remote-frontend/src/lib/api/tasks.ts
  - remote-frontend/src/pages/Tasks.test.tsx
irreversible: false
scope_test: "remote-frontend/src/pages/Tasks.test.tsx"
allowed_change: mixed
covers_criteria: [SC2]
---
## Failing test (write first)

Append to `remote-frontend/src/pages/Tasks.test.tsx`:

```tsx
import { tasksApi } from '@/lib/api/tasks';

vi.mock('@/lib/api/tasks', () => ({
  tasksApi: {
    setExecutingNode: vi.fn(() => Promise.resolve({ ok: true })),
    delete: vi.fn(() => Promise.resolve({ ok: true })),
    patch: vi.fn(() => Promise.resolve({ ok: true })),
  },
}));

describe('TasksBoard management actions', () => {
  it('calls PATCH /v1/tasks/{id}/executing-node when Assign clicked', async () => {
    render(<TasksBoard />);
    const assignBtn = await screen.findByRole('button', { name: /assign/i });
    await act(async () => { fireEvent.click(assignBtn); });
    expect(tasksApi.setExecutingNode).toHaveBeenCalledWith('t1', expect.anything());
  });

  it('calls DELETE /v1/tasks/{id} when Delete clicked', async () => {
    render(<TasksBoard />);
    const deleteBtn = await screen.findByRole('button', { name: /delete/i });
    await act(async () => { fireEvent.click(deleteBtn); });
    expect(tasksApi.delete).toHaveBeenCalledWith('t1');
  });
});
```

Test fails red — `tasksApi` not yet defined.

## Change

### File: `remote-frontend/src/lib/api/tasks.ts` (CREATE)

**Sibling alignment:** Read `remote-frontend/src/lib/api/utils.ts` (the API client helpers — `makeRequest`, `handleApiResponse`, `ApiError`). Use the same helpers, NOT fetch directly. Justify any divergence in the decisions ledger.

The hive REST routes (per `crates/remote/src/routes/tasks.rs`, nested under `/v1` per `crates/remote/src/routes/mod.rs:112-113`):
- `POST /v1/tasks/{task_id}/assign` — body: `{ new_assignee_user_id, version }` → assigns task to a **human user** (GitHub-style assignee). NOT used by the board's node-assignment action.
- `PATCH /v1/tasks/{task_id}/executing-node` — body: `{ node_id }` → sets/reassigns the **executing node**. This is the board's "assign to node" action.
- `DELETE /v1/tasks/{task_id}` — deletes the task.
- `POST /v1/tasks` — create task (not needed for the board's actions; included for completeness).

**Decision (record in ledger):** The board's "assign" action targets a NODE (which node runs the task), NOT a human user. The correct route is `PATCH /tasks/{id}/executing-node` with `{ node_id }`. The `POST /tasks/{id}/assign` route is for human-user assignment (`{ new_assignee_user_id, version }`) and is out of scope for the cross-node board (the spec scopes SC2 to cross-node aggregation views, not human-assignee management). If a future task needs human-assignee UI, add a separate `tasksApi.assignToUser(taskId, userId, version)` method then.

```ts
import { makeRequest, handleApiResponse } from './utils';

const API_BASE = import.meta.env.VITE_API_BASE_URL || '/v1';

export const tasksApi = {
  setExecutingNode: (taskId: string, nodeId: string) =>
    makeRequest(`${API_BASE}/tasks/${taskId}/executing-node`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ node_id: nodeId }),
    }).then(handleApiResponse),

  delete: (taskId: string) =>
    makeRequest(`${API_BASE}/tasks/${taskId}`, {
      method: 'DELETE',
    }).then(handleApiResponse),
};
```

### File: `remote-frontend/src/pages/Tasks.tsx`

In `TasksBoard`, add management action buttons to each assignment row:
- **Set Executing Node:** calls `tasksApi.setExecutingNode(a.task_id, selectedNodeId)` — opens a node picker (reuse `nodes` collection data). Label the button "Assign to node" (the action is "set the executing node", which is semantically "which node runs this task").
- **Delete:** calls `tasksApi.delete(a.task_id)` with a confirm.

On success, the Electric collection reactively updates (no manual refetch — the shape poll picks up the DB change). Do NOT add a `useQuery` refetch — that would re-introduce REST reads, contradicting SC5.

### File: `remote-frontend/src/pages/Tasks.test.tsx`

Append the management-actions describe block + the `tasksApi` mock.

## Allowed moves
- CREATE `remote-frontend/src/lib/api/tasks.ts` with `tasksApi` (setExecutingNode/delete).
- Add action buttons + handlers to `TasksBoard` in `remote-frontend/src/pages/Tasks.tsx`.
- Extend the test file.
- No server changes (routes exist in `crates/remote/src/routes/tasks.rs`).
- No new push channels.

## STOP triggers
- The hive's `PATCH /v1/tasks/{id}/executing-node` route expects a different body shape than `{ node_id }` — HALT; verify against `crates/remote/src/routes/tasks.rs:618-647` (`SetExecutingNodeRequest`) and record the actual contract in the ledger. (Cross-checked: the body IS `{ node_id }`; `SetExecutingNodeRequest{ node_id: Option<Uuid> }`.)
- `makeRequest` / `handleApiResponse` signatures in `remote-frontend/src/lib/api/utils.ts` differ from node's — HALT; adapt the client to the hive's helpers (record in ledger).

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/pages/Tasks.test.tsx
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
```
All exit 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Tasks.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 307` exits 0