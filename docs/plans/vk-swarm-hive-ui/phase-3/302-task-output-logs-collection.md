---
id: "302"
phase: 3
title: Add ElectricTaskOutputLog type + createTaskOutputLogsCollection
status: ready
depends_on: ["300"]
parallel: false
conflicts_with: ["301", "303"]
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

Add to `remote-frontend/src/lib/electric/collections.test.ts`, in a new `describe('createTaskOutputLogsCollection', ...)` block, mirroring the existing `createNodeProjectsCollection` block structure:

```ts
describe('createTaskOutputLogsCollection', () => {
  it('creates a collection with correct shape URL', () => {
    const collection = createTaskOutputLogsCollection();

    expect(electricCollectionOptions).toHaveBeenCalledWith(
      expect.objectContaining({
        shapeOptions: expect.objectContaining({
          url: '/api/electric/v1/shape/node_task_output_logs',
        }),
      })
    );
    expect(createCollection).toHaveBeenCalled();
    expect(collection).toBeDefined();
  });

  it('uses id as the key extractor', () => {
    createTaskOutputLogsCollection();

    const config = (electricCollectionOptions as ReturnType<typeof vi.fn>)
      .mock.calls[0][0] as ElectricCollectionConfig<{ id: string }>;
    expect(config.getKey({ id: 'log-1' })).toBe('log-1');
  });
});
```

Add `createTaskOutputLogsCollection` to the import in the remote-frontend test file. Test fails red (symbol not exported from the remote-frontend copy).

**BIGSERIAL PK note (Claude F11):** `node_task_output_logs.id` is `BIGSERIAL PRIMARY KEY` (`migrations/20251202000001_task_output_logs.sql:3`), not UUID. The type below annotates `id: string`. If `@tanstack/electric-db-collection` serializes BIGSERIAL as a JS `number` at runtime, the `string` annotation is a TypeScript lie and `getKey` returns the wrong type. STOP and verify how the Electric client serializes BIGSERIAL before pinning the type; default to `id: string | number` until verified, and record the decision in the ledger.

**Spec divergence note (record in ledger):** The spec's `## Design` field names diverge from the DB schema (see task 301 for the full list). This task uses DB-accurate names: `output_type` (not `stream`), `content`/`message`+`metadata` (not `payload`).

## Change

### File: `remote-frontend/src/lib/electric/collections.ts`

**Sibling alignment:** Read `remote-frontend/src/lib/electric/collections.ts` (the copy task 300 created) AND `frontend/src/lib/electric/collections.ts` (read-only reference — do NOT edit it; SC4). The new collection MUST match the existing 3 collection creators' shape. Justify any divergence in the ledger.

**Anchor:** after the last collection function in the file (location depends on whether 301 or 302 lands first — append at EOF regardless of order).

**After:** append (do NOT duplicate `ElectricTaskAssignment` if 301 already landed):

```ts
/**
 * Task-output-log type for Electric sync.
 * Matches the PostgreSQL node_task_output_logs table structure.
 * Schema: crates/remote/migrations/20251202000001_task_output_logs.sql
 *         + 20251229100000_sync_task_attempts.sql:47-48 (execution_process_id).
 */
export type ElectricTaskOutputLog = ElectricRow & {
  id: string;
  assignment_id: string;
  output_type: string; // 'stdout', 'stderr', 'system'
  content: string;
  timestamp: string;
  created_at: string;
  execution_process_id: string | null;
};

/**
 * Create a collection for node-task output logs.
 * Syncs task execution stdout/stderr/system logs from the hive.
 *
 * @returns TanStack DB collection for node-task output logs
 */
export function createTaskOutputLogsCollection() {
  return createCollection(
    electricCollectionOptions<ElectricTaskOutputLog>({
      shapeOptions: {
        url: createShapeUrl('node_task_output_logs'),
      },
      getKey: (item) => item.id,
    })
  );
}
```

## Allowed moves
- Append the `ElectricTaskOutputLog` type and `createTaskOutputLogsCollection` function to `remote-frontend/src/lib/electric/collections.ts`.
- Append the `describe('createTaskOutputLogsCollection', ...)` block + add `createTaskOutputLogsCollection` to the import in `remote-frontend/src/lib/electric/collections.test.ts`.
- Read-only reference to `frontend/src/lib/electric/collections.ts` (SC4 — do NOT edit it).
- No other file. Do NOT edit `remote-frontend/src/lib/electric/index.ts` (task 304).

## STOP triggers
- The `node_task_output_logs` table is missing from `ELECTRIC_SHAPE_TABLES` in `config.ts` — HALT (present per `config.ts:80-83`; verify).
- A prior task already added `ElectricTaskOutputLog` (deduplication across 301/302/303) — skip the type, keep only the function.

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/lib/electric/collections.test.ts
cd remote-frontend && npx tsc --noEmit
cd frontend && npx tsc --noEmit   # SC4 — node frontend untouched
```
All exit 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/collections.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 302` exits 0