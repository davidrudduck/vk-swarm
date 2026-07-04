---
id: "301"
phase: 3
title: Add ElectricTaskAssignment type + createTaskAssignmentsCollection
status: ready
depends_on: ["300"]
parallel: false
conflicts_with: ["302", "303"]
files:
  - frontend/src/lib/electric/collections.ts
  - frontend/src/lib/electric/collections.test.ts
  - remote-frontend/src/lib/electric/collections.ts
  - remote-frontend/src/lib/electric/collections.test.ts
irreversible: false
scope_test: "remote-frontend/src/lib/electric/collections.test.ts"
allowed_change: edit
covers_criteria: [SC5]
---
## Failing test (write first)

Add to `remote-frontend/src/lib/electric/collections.test.ts`, in a new `describe('createTaskAssignmentsCollection', ...)` block after the existing `createNodeProjectsCollection` block, mirroring its structure exactly:

```ts
describe('createTaskAssignmentsCollection', () => {
  it('creates a collection with correct shape URL', () => {
    const collection = createTaskAssignmentsCollection();

    expect(electricCollectionOptions).toHaveBeenCalledWith(
      expect.objectContaining({
        shapeOptions: expect.objectContaining({
          url: '/api/electric/v1/shape/node_task_assignments',
        }),
      })
    );
    expect(createCollection).toHaveBeenCalled();
    expect(collection).toBeDefined();
  });

  it('uses id as the key extractor', () => {
    createTaskAssignmentsCollection();

    const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
      .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
    expect(config.getKey({ id: 'assignment-uuid' })).toBe('assignment-uuid');
  });
});
```

Also add `createTaskAssignmentsCollection` to the existing import in the remote-frontend test file. The test fails red because `createTaskAssignmentsCollection` is not yet exported from `remote-frontend/src/lib/electric/collections.ts` (task 300 created the byte-identical copy; this task adds the new collection to it).

## Change

### File: `remote-frontend/src/lib/electric/collections.ts`

**Sibling alignment:** Read `remote-frontend/src/lib/electric/collections.ts` (the copy task 300 created) AND `frontend/src/lib/electric/collections.ts` (the node-frontend original — read-only reference). The new collection MUST match the existing 3 collection creators' shape exactly. Justify any divergence in the decisions ledger. Do NOT edit `frontend/src/lib/electric/collections.ts` (SC4 — node frontend untouched).

**Spec divergence note (record in ledger):** The spec's `## Design` lists Electric type field names that DIVERGE from the actual DB schema. The task uses the DB-accurate field names (from `migrations/20251202000000_nodes_swarm.sql:73-85` + `20260128000001_add_lease_fencing.sql`): `execution_status` (not `status`), `lease_expires_at` (not `leased_until`), `assignment_id`/`task_id`/`node_id` (not `task_id`/`node_id` only), `output_type` (not `stream`), `message`+`metadata` (not `payload`). The spec's names are aspirational; the DB is the source of truth. This divergence is recorded here so the next reviewer doesn't re-flag it.

**Anchor:** after the `createNodeProjectsCollection` function (line 121), before EOF.

**Before:**
```ts
export function createNodeProjectsCollection() {
  return createCollection(
    electricCollectionOptions<ElectricNodeProject>({
      shapeOptions: {
        url: createShapeUrl('node_projects'),
      },
      getKey: (item) => item.id,
    })
  );
}
```

**After:** append the new type + collection:

```ts
export function createNodeProjectsCollection() {
  return createCollection(
    electricCollectionOptions<ElectricNodeProject>({
      shapeOptions: {
        url: createShapeUrl('node_projects'),
      },
      getKey: (item) => item.id,
    })
  );
}

/**
 * Task-assignment type for Electric sync.
 * Matches the PostgreSQL node_task_assignments table structure.
 * Schema: crates/remote/migrations/20251202000000_nodes_swarm.sql:73-85
 *         + 20260128000001_add_lease_fencing.sql (lease_expires_at, fencing_token).
 */
export type ElectricTaskAssignment = ElectricRow & {
  id: string;
  task_id: string;
  node_id: string;
  node_project_id: string;
  local_task_id: string | null;
  local_attempt_id: string | null;
  execution_status: string;
  assigned_at: string;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
  lease_expires_at: string | null;
  fencing_token: number;
};

/**
 * Create a collection for node-task assignments.
 * Syncs task execution assignments from the hive (which node runs which task).
 *
 * @returns TanStack DB collection for node-task assignments
 */
export function createTaskAssignmentsCollection() {
  return createCollection(
    electricCollectionOptions<ElectricTaskAssignment>({
      shapeOptions: {
        url: createShapeUrl('node_task_assignments'),
      },
      getKey: (item) => item.id,
    })
  );
}
```

## Allowed moves
- Append the `ElectricTaskAssignment` type and `createTaskAssignmentsCollection` function to `remote-frontend/src/lib/electric/collections.ts`.
- Append the `describe('createTaskAssignmentsCollection', ...)` block + add `createTaskAssignmentsCollection` to the import in `remote-frontend/src/lib/electric/collections.test.ts`.
- Read-only reference to `frontend/src/lib/electric/collections.ts` (the sibling — do NOT edit it; SC4).
- No other file. Do NOT edit `remote-frontend/src/lib/electric/index.ts` (task 304 owns the export wiring).

## STOP triggers
- The `node_task_assignments` table name is missing from `ELECTRIC_SHAPE_TABLES` in `config.ts` — HALT; the shape config was assumed present but isn't. (It is present per `config.ts:72-75`; verify before halting.)
- `createShapeUrl('node_task_assignments')` produces a URL not starting with `ELECTRIC_PROXY_BASE` — HALT; config regression.

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/lib/electric/collections.test.ts
cd remote-frontend && npx tsc --noEmit
cd frontend && npx tsc --noEmit   # SC4 — node frontend untouched, still green
```
All exit 0. The new test block passes; the existing 3 collection tests still pass; tsc has no new errors in either frontend.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/collections.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 301` exits 0