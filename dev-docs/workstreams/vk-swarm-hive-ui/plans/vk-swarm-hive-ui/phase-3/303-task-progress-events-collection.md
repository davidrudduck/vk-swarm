---
id: "303"
phase: 3
title: Add ElectricTaskProgressEvent type + createTaskProgressEventsCollection
status: done
depends_on: ["300"]
parallel: false
conflicts_with: ["301", "302"]
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

Add to `remote-frontend/src/lib/electric/collections.test.ts`, in a new `describe('createTaskProgressEventsCollection', ...)` block, mirroring the existing `createNodeProjectsCollection` block:

```ts
describe('createTaskProgressEventsCollection', () => {
  it('creates a collection with correct shape URL', () => {
    const collection = createTaskProgressEventsCollection();

    expect(electricCollectionOptions).toHaveBeenCalledWith(
      expect.objectContaining({
        shapeOptions: expect.objectContaining({
          url: '/api/electric/v1/shape/node_task_progress_events',
        }),
      })
    );
    expect(createCollection).toHaveBeenCalled();
    expect(collection).toBeDefined();
  });

  it('uses id as the key extractor', () => {
    createTaskProgressEventsCollection();

    const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
      .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
    expect(config.getKey({ id: 'event-1' })).toBe('event-1');
  });
});
```

Add `createTaskProgressEventsCollection` to the import in the remote-frontend test file. Test fails red (not exported from the remote-frontend copy).

**BIGSERIAL PK note (Claude F11):** `node_task_progress_events.id` is `BIGSERIAL PRIMARY KEY` (`migrations/20251202000002_task_progress_events.sql:3`). See task 302's BIGSERIAL note — same applies here. Default to `id: string | number` until verified; record the decision in the ledger.

**Spec divergence note (record in ledger):** The spec's `## Design` field names diverge from the DB schema (see task 301). This task uses DB-accurate names: `event_type`/`message`+`metadata` (not `stream`/`payload`).

## Change

### File: `remote-frontend/src/lib/electric/collections.ts`

**Sibling alignment:** Read `remote-frontend/src/lib/electric/collections.ts` (task 300's copy) AND `frontend/src/lib/electric/collections.ts` (read-only; SC4). Match the existing `createCollection(electricCollectionOptions<Type>({ shapeOptions: { url: createShapeUrl('<table>') }, getKey: (item) => item.id }))` shape exactly. Justify any divergence in the decisions ledger. Do NOT edit the node frontend's copy.

**Anchor:** after the last collection function (append at EOF regardless of 301/302 ordering).

**After:** append:

```ts
/**
 * Task-progress-event type for Electric sync.
 * Matches the PostgreSQL node_task_progress_events table structure.
 * Schema: crates/remote/migrations/20251202000002_task_progress_events.sql
 */
export type ElectricTaskProgressEvent = ElectricRow & {
  id: string;
  assignment_id: string;
  event_type: string; // 'agent_started', 'branch_created', 'committed', etc.
  message: string | null;
  metadata: unknown | null; // JSONB
  timestamp: string;
  created_at: string;
};

/**
 * Create a collection for node-task progress events.
 * Syncs task execution progress milestones from the hive.
 *
 * @returns TanStack DB collection for node-task progress events
 */
export function createTaskProgressEventsCollection() {
  return createCollection(
    electricCollectionOptions<ElectricTaskProgressEvent>({
      shapeOptions: {
        url: createShapeUrl('node_task_progress_events'),
      },
      getKey: (item) => item.id,
    })
  );
}
```

## Allowed moves
- Append the `ElectricTaskProgressEvent` type and `createTaskProgressEventsCollection` function to `remote-frontend/src/lib/electric/collections.ts`.
- Append the `describe('createTaskProgressEventsCollection', ...)` block + add `createTaskProgressEventsCollection` to the import in `remote-frontend/src/lib/electric/collections.test.ts`.
- Read-only reference to `frontend/src/lib/electric/collections.ts` (SC4 — do NOT edit it).
- No other file. Do NOT edit `remote-frontend/src/lib/electric/index.ts` (task 304).

## STOP triggers
- The `node_task_progress_events` table is missing from `ELECTRIC_SHAPE_TABLES` in `config.ts` — HALT (present per `config.ts:88-91`; verify).
- A prior task already added `ElectricTaskProgressEvent` — skip the type, keep only the function.

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/lib/electric/collections.test.ts
cd remote-frontend && npx tsc --noEmit
cd frontend && npx tsc --noEmit   # SC4 — node frontend untouched
```
All exit 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/collections.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 303` exits 0