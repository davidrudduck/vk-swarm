---
id: "201"
phase: 2
title: "Rehost setup: add @/* and shared/* path aliases + copy shared types into remote-frontend"
status: ready
depends_on: ["106"]
parallel: false
conflicts_with: []
files:
  - frontend/tsconfig.json
  - remote-frontend/tsconfig.json
  - remote-frontend/vite.config.ts
  - shared/types.ts
  - remote-frontend/src/types/shared/types.ts
irreversible: false
scope_test: "remote-frontend/src/types/shared.test.ts"
allowed_change: mixed
covers_criteria: [SC1]
---
## Failing test (write first)
File: `remote-frontend/src/types/shared.test.ts`

Imports a type from the `shared/*` alias path (e.g. `import type { ApiResponse } from 'shared/types'`) and asserts it compiles + the type is the expected shape. This is a compile-time test — if `tsc --noEmit` passes, the alias resolves. The vitest test can be minimal: `import { describe, it, expect } from 'vitest'; import type { ApiResponse } from 'shared/types'; describe('shared alias', () => { it('resolves', () => { const x: ApiResponse<string> = { data: 'ok', status: 200 }; expect(x.data).toBe('ok'); }); });`

## Change
- **File:** `remote-frontend/tsconfig.json` (EDIT)
  - **Anchor:** the `compilerOptions` object.
  - **Before:** no `paths` key (the hive tsconfig has no path aliases).
  - **After:** add:
    ```jsonc
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"],
      "shared/*": ["src/types/shared/*"]
    }
    ```
    NOTE: the node `frontend/tsconfig.json` maps `shared/*` to `["../shared/*"]` (the repo-root `shared/` dir). The hive `remote-frontend/` has no sibling `shared/` dir in its include path, so we map `shared/*` to `src/types/shared/*` and COPY the shared types there (next file). This keeps the swarm API clients' `import { X } from 'shared/types'` statements UNCHANGED when copied in task 202.
  - **Sibling alignment:** Read `frontend/tsconfig.json`. It maps `@/*` → `src/*` (same) and `shared/*` → `../shared/*` (different — node has repo-root `shared/`, hive copies into `src/types/shared/`). Justify the divergence in the ledger: the hive app is self-contained; it does not reach across the repo into `shared/`. The copy is a one-time snapshot; drift is reconciled by extracting to a shared package later (spec decision 2).

- **File:** `remote-frontend/src/types/shared/types.ts` (CREATE — copy)
  - **Before:** (file does not exist)
  - **After:** Copy `shared/types.ts` verbatim. This is the shared types module the swarm API clients import (`ApiResponse`, `StatusResponse`, etc.).
  - **Sibling alignment:** Read `shared/types.ts`. List every export. The copy must be byte-identical. If `shared/types.ts` imports from other files (e.g. `shared/schemas/`), those must ALSO be copied into `src/types/shared/` — recurse and copy the full dependency closure. Record what was copied in the ledger.

- **File:** `remote-frontend/src/vite.config.ts` (EDIT — likely needed)
  - **Anchor:** the Vite config `resolve` block (or add one).
  - **Before:** no `resolve.alias` (or only the default).
  - **After:** add `resolve.alias` mapping `@` → `./src` and `shared` → `./src/types/shared` to match the tsconfig paths. Vite needs this for dev/build resolution (tsconfig paths are TS-only; Vite uses its own resolver).
  - If `remote-frontend/vite.config.ts` does not exist, find the actual Vite config file (`vite.config.js`, `vite.config.mts`, etc.) and edit that.

## Allowed moves
- Edit `remote-frontend/tsconfig.json`, `remote-frontend/vite.config.*`, create `remote-frontend/src/types/shared/*`.
- Read-only reference to `frontend/tsconfig.json` and `shared/types.ts`.

## STOP triggers
- If `shared/types.ts` imports from outside `shared/` (a non-relative import that isn't a path alias) — STOP; the dependency closure is wider than expected. Record the import in the ledger and either copy the missing module too or rewrite the import.
- If `remote-frontend/` does not use Vite (uses webpack/rollup/etc.) — STOP; the alias setup is bundler-specific. Record the actual bundler in the ledger and adjust.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/types/shared.test.ts` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0 (the alias resolves).
- `cd remote-frontend && npm run lint` exits 0.
- `cd remote-frontend && npm run build` exits 0 (Vite resolves the alias at build time).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/types/shared.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 201` exits 0