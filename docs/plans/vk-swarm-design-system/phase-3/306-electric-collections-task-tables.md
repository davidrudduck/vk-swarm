---
id: "306"
phase: 3
title: Electric collections for task tables (copy + 3 new collections)
status: passed
depends_on: ["106","201"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/lib/electric/config.ts
  - remote-frontend/src/lib/electric/collections.ts
  - remote-frontend/src/lib/electric/index.ts
  - remote-frontend/src/lib/electric/electric.test.ts
irreversible: false
scope_test: "remote-frontend/src/lib/electric/electric.test.ts"
allowed_change: create
covers_criteria: [SC8]
---

## Sibling alignment

Read `frontend/src/lib/electric/{config,collections,index}.ts` (node frontend). `config.ts` defines `ELECTRIC_PROXY_BASE = '/api/electric/v1/shape'` (node proxy path) + `ELECTRIC_SHAPE_TABLES` (6 tables: nodes, projects, node_projects, node_task_assignments, node_task_output_logs, node_task_progress_events) + `createShapeUrl(table)`. `collections.ts` defines 3 collections (nodes, projects, node_projects) using `createCollection` from `@tanstack/react-db` + `electricCollectionOptions` from `@tanstack/electric-db-collection` + types `ElectricNode`, `ElectricProject`, `ElectricNodeProject` (all `ElectricRow & {...}`). The COPY to `remote-frontend/src/lib/electric/` must:
1. Repoint `ELECTRIC_PROXY_BASE` from `/api/electric/v1/shape` (node proxy) to `/v1/shape` (hive proxy per `crates/remote/src/routes/mod.rs:112-113` + `electric_proxy.rs:28`).
2. Add 3 new collections: `createTaskAssignmentsCollection()`, `createTaskOutputLogsCollection()`, `createTaskProgressEventsCollection()` with types `ElectricTaskAssignment`, `ElectricTaskOutputLog`, `ElectricTaskProgressEvent`.
3. Re-export all 6 collections + all 6 types from `index.ts`.

**KNOWN GAP — record in ledger:** The hive electric proxy only exposes `GET /v1/shape/shared_tasks` (`crates/remote/src/routes/electric_proxy.rs:28`). The other 5 shape tables (nodes, projects, node_projects, node_task_assignments, node_task_output_logs, node_task_progress_events) are NOT exposed via the hive proxy. The collections will compile + the types are correct, but at runtime the shape requests for the 5 non-shared_tasks tables will 404 against the hive. Task 308 (data wiring) must account for this — BoardView will consume `tasksApi.bulk()` (REST) as the primary data source, with Electric collections as a progressive-enhancement layer that activates when the hive adds more shape routes. Record this gap in the ledger.

## Failing test (write first)

Create `remote-frontend/src/lib/electric/electric.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { ELECTRIC_PROXY_BASE, ELECTRIC_SHAPE_TABLES, createShapeUrl, type ElectricShapeTable } from './index';
import { createNodesCollection, createProjectsCollection, createNodeProjectsCollection,
         createTaskAssignmentsCollection, createTaskOutputLogsCollection, createTaskProgressEventsCollection,
         type ElectricNode, type ElectricProject, type ElectricNodeProject,
         type ElectricTaskAssignment, type ElectricTaskOutputLog, type ElectricTaskProgressEvent } from './index';

describe('electric config (SC8)', () => {
  it('ELECTRIC_PROXY_BASE points at the hive proxy (/v1/shape), not the node proxy', () => {
    expect(ELECTRIC_PROXY_BASE).toBe('/v1/shape');
  });
  it('ELECTRIC_SHAPE_TABLES has 6 tables', () => {
    const keys = Object.keys(ELECTRIC_SHAPE_TABLES);
    expect(keys).toHaveLength(6);
    expect(keys).toContain('node_task_assignments');
    expect(keys).toContain('node_task_output_logs');
    expect(keys).toContain('node_task_progress_events');
  });
  it('createShapeUrl produces hive-proxy URLs', () => {
    expect(createShapeUrl('nodes')).toBe('/v1/shape/nodes');
    expect(createShapeUrl('node_task_assignments')).toBe('/v1/shape/node_task_assignments');
  });
});

describe('electric collections (SC8)', () => {
  it('all 6 collection factories are functions', () => {
    expect(typeof createNodesCollection).toBe('function');
    expect(typeof createProjectsCollection).toBe('function');
    expect(typeof createNodeProjectsCollection).toBe('function');
    expect(typeof createTaskAssignmentsCollection).toBe('function');
    expect(typeof createTaskOutputLogsCollection).toBe('function');
    expect(typeof createTaskProgressEventsCollection).toBe('function');
  });
  it('new types extend ElectricRow', () => {
    const a: ElectricTaskAssignment = { id: 'a', assignment_id: 'x', task_id: 't', node_id: 'n', execution_status: 'pending', lease_expires_at: null };
    const o: ElectricTaskOutputLog = { id: 1, assignment_id: 'x', output_type: 'stdout', message: 'm', metadata: null, created_at: 't' };
    const p: ElectricTaskProgressEvent = { id: 1, assignment_id: 'x', event_type: 'e', message: 'm', created_at: 't' };
    expect(a.id).toBe('a');
    expect(o.id).toBe(1);
    expect(p.id).toBe(1);
  });
});
```

## Change

### File: `remote-frontend/src/lib/electric/config.ts` (CREATE)
Byte-for-byte copy of `frontend/src/lib/electric/config.ts` EXCEPT line 1 of the const: `export const ELECTRIC_PROXY_BASE = '/v1/shape';` (was `'/api/electric/v1/shape'`). Keep `ELECTRIC_SHAPE_TABLES` (6 tables), `createShapeUrl`, `getElectricBaseUrl`, `createShapeStreamOptions`, all types.

### File: `remote-frontend/src/lib/electric/collections.ts` (CREATE)
Copy the 3 existing collections (nodes, projects, node_projects) + their types verbatim (they already use `electricCollectionOptions({ shapeOptions: { url: createShapeUrl(...) }, getKey: (item) => item.id })`). ADD 3 new collections:
- `ElectricTaskAssignment = ElectricRow & { id: string; assignment_id: string; task_id: string; node_id: string; execution_status: string; lease_expires_at: string | null }` + `createTaskAssignmentsCollection()` using `electricCollectionOptions({ shapeOptions: { url: createShapeUrl('node_task_assignments') }, getKey: (item) => item.assignment_id })`.
- `ElectricTaskOutputLog = ElectricRow & { id: string | number; assignment_id: string; output_type: string; message: string; metadata: string | null; created_at: string }` + `createTaskOutputLogsCollection()` using `electricCollectionOptions({ shapeOptions: { url: createShapeUrl('node_task_output_logs') }, getKey: (item) => item.id })`.
- `ElectricTaskProgressEvent = ElectricRow & { id: string | number; assignment_id: string; event_type: string; message: string; created_at: string }` + `createTaskProgressEventsCollection()` using `electricCollectionOptions({ shapeOptions: { url: createShapeUrl('node_task_progress_events') }, getKey: (item) => item.id })`.

### File: `remote-frontend/src/lib/electric/index.ts` (CREATE)
Re-export everything from `./config` + `./collections` (all 6 collection factories + all 6 types + config exports).

### File: `remote-frontend/src/lib/electric/electric.test.ts` (CREATE)
Create exactly as written in Failing test above.

## Allowed moves

- Create the 4 files as specified.
- Copy verbatim from `frontend/src/lib/electric/` except the `ELECTRIC_PROXY_BASE` repoint.
- Add the 3 new collections + types.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `frontend/src/lib/electric/collections.ts` uses an API shape incompatible with `@tanstack/electric-db-collection@^0.3.12` installed in task 100.
- The hive electric proxy route differs from `GET /v1/shape/shared_tasks` (would change the repoint strategy → STOP, escalate).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/electric.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 306` exits 0.