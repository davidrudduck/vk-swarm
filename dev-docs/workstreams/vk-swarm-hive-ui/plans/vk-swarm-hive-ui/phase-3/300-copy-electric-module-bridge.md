---
id: "300"
phase: 3
title: "Copy frontend/src/lib/electric/ into remote-frontend/src/lib/electric/ (alias bridge for @/lib/electric)"
status: done
depends_on: ["106"]
parallel: false
conflicts_with: []
files:
  - frontend/src/lib/electric/config.ts
  - frontend/src/lib/electric/collections.ts
  - frontend/src/lib/electric/index.ts
  - frontend/src/lib/electric/config.test.ts
  - frontend/src/lib/electric/collections.test.ts
  - remote-frontend/src/lib/electric/config.ts
  - remote-frontend/src/lib/electric/collections.ts
  - remote-frontend/src/lib/electric/index.ts
  - remote-frontend/src/lib/electric/config.test.ts
  - remote-frontend/src/lib/electric/collections.test.ts
  - remote-frontend/src/lib/electric/bridge.test.ts
irreversible: false
scope_test: "remote-frontend/src/lib/electric/bridge.test.ts"
allowed_change: create
covers_criteria: [SC5]
---
## Failing test (write first)
File: `remote-frontend/src/lib/electric/bridge.test.ts`

```ts
import { describe, it, expect } from 'vitest';
import { ELECTRIC_PROXY_BASE, ELECTRIC_SHAPE_TABLES, createShapeUrl } from './config';
import { createNodesCollection, createProjectsCollection, createNodeProjectsCollection } from './collections';

describe('electric bridge', () => {
  it('config is importable from remote-frontend', () => {
    expect(typeof ELECTRIC_PROXY_BASE).toBe('string');
    expect(Array.isArray(ELECTRIC_SHAPE_TABLES)).toBe(true);
    expect(typeof createShapeUrl).toBe('function');
  });
  it('existing collections are importable', () => {
    expect(typeof createNodesCollection).toBe('function');
    expect(typeof createProjectsCollection).toBe('function');
    expect(typeof createNodeProjectsCollection).toBe('function');
  });
});
```

This fails red because `remote-frontend/src/lib/electric/` does not exist (no `lib/` dir under `remote-frontend/src/`). Tasks 301-305 import `@/lib/electric` which resolves to `remote-frontend/src/lib/electric/` — this task creates that module by copying from `frontend/src/lib/electric/`.

## Change
- **File:** `remote-frontend/src/lib/electric/config.ts` (CREATE — copy)
  - **Before:** (file does not exist)
  - **After:** Byte-identical copy of `frontend/src/lib/electric/config.ts`. Contains `ELECTRIC_PROXY_BASE`, `ELECTRIC_SHAPE_TABLES` (6 tables), `createShapeUrl`.
- **File:** `remote-frontend/src/lib/electric/collections.ts` (CREATE — copy)
  - **Before:** (file does not exist)
  - **After:** Byte-identical copy of `frontend/src/lib/electric/collections.ts`. Contains the 3 existing collections (nodes, projects, node_projects) + their types (ElectricNode, ElectricProject, ElectricNodeProject). Tasks 301-303 will ADD the 3 new collections + types to this file.
- **File:** `remote-frontend/src/lib/electric/index.ts` (CREATE — copy)
  - **Before:** (file does not exist)
  - **After:** Byte-identical copy of `frontend/src/lib/electric/index.ts`. Re-exports config + collections. Task 304 will extend it.
- **Sibling alignment:** Read `frontend/src/lib/electric/{config,collections,index}.ts`. The copy must be byte-identical. The node frontend's copy is the HA-fallback source of truth; the hive copy is what tasks 301-305 edit. Record in the ledger that the node frontend copy is NOT modified by this workstream (SC4). The two copies will drift once tasks 301-303 add new collections to the hive copy only — this is intended (the hive is the new home; the node frontend stays as fallback).

## Allowed moves
- Copy `frontend/src/lib/electric/{config,collections,index}.ts` → `remote-frontend/src/lib/electric/{config,collections,index}.ts`.
- Copy the existing test files `frontend/src/lib/electric/{config,collections}.test.ts` → `remote-frontend/src/lib/electric/` (so the copied module has its own test coverage).
- Create `remote-frontend/src/lib/electric/bridge.test.ts`.
- No edits to `frontend/src/lib/electric/*` (read-only sibling reference; SC4).

## STOP triggers
- If `frontend/src/lib/electric/collections.ts` imports from outside `frontend/src/lib/electric/` (e.g. a relative import to `../api/...` or a `shared/` alias) — STOP; the copy has a dependency closure wider than the module. Record the import and either copy the missing dep too or rewrite the import to a hive-local path.
- If `@tanstack/react-db` or `@tanstack/electric-db-collection` is not installed in `remote-frontend/package.json` — STOP; task 100 must run first.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/lib/electric/bridge.test.ts` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.
- `cd frontend && npx tsc --noEmit` exits 0 (node frontend untouched — SC4).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/electric/bridge.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 300` exits 0