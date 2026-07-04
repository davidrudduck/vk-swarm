---
id: "100"
phase: 1
title: Install remote-frontend toolchain (vitest, eslint, @tanstack/react-db, testing-library)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - remote-frontend/package.json
  - remote-frontend/package-lock.json
  - remote-frontend/eslint.config.js
  - remote-frontend/vite.config.ts
  - remote-frontend/tsconfig.json
  - remote-frontend/src/setupTests.ts
  - remote-frontend/src/toolchain.test.ts
irreversible: false
scope_test: "remote-frontend/src/toolchain.test.ts"
allowed_change: mixed
covers_criteria: [SC3]
---
## Failing test (write first)

Create `remote-frontend/src/toolchain.test.ts`:

```ts
import { describe, it, expect } from 'vitest';

describe('toolchain', () => {
  it('vitest is importable', () => {
    expect(typeof describe).toBe('function');
    expect(typeof it).toBe('function');
    expect(typeof expect).toBe('function');
  });
});
```

This fails red because `vitest` is not installed (the import resolves to nothing). Once the deps are installed, `npx vitest run src/toolchain.test.ts` passes.

## Change

### File: `remote-frontend/package.json` (EDIT)

**Sibling alignment:** Read `frontend/package.json` for the exact dep versions used by the node frontend. The hive frontend MUST use compatible versions (same major for react/react-dom; same for @tanstack/react-db + @tanstack/react-query + @tanstack/electric-db-collection so the Electric collections compile unchanged when copied in task 201/305). Justify any divergence in the decisions ledger.

**Before:**
```json
{
  "name": "remote-frontend",
  "private": true,
  "version": "0.0.1",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^7.9.5"
  },
  "devDependencies": {
    "@types/react": "^18.2.43",
    "@types/react-dom": "^18.2.17",
    "@vitejs/plugin-react": "^4.2.1",
    "autoprefixer": "^10.4.16",
    "postcss": "^8.4.32",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.9.2",
    "vite": "^5.0.8"
  }
}
```

**After:** add lint/test scripts + the runtime deps the downstream tasks need + the dev deps for the toolchain:

```json
{
  "name": "remote-frontend",
  "private": true,
  "version": "0.0.1",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "lint": "eslint src --max-warnings 0",
    "test": "vitest",
    "test:run": "vitest run"
  },
  "dependencies": {
    "@tanstack/electric-db-collection": "^0.3.12",
    "@tanstack/react-db": "^0.1.82",
    "@tanstack/react-query": "^5.96.2",
    "clsx": "^2.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^7.9.5",
    "tailwind-merge": "^2.2.0"
  },
  "devDependencies": {
    "@eslint/js": "^10.0.1",
    "@testing-library/dom": "^10.4.1",
    "@testing-library/jest-dom": "^6.9.1",
    "@testing-library/react": "^16.3.2",
    "@types/react": "^18.2.43",
    "@types/react-dom": "^18.2.17",
    "@typescript-eslint/eslint-plugin": "^8.58.1",
    "@typescript-eslint/parser": "^8.58.1",
    "@vitejs/plugin-react": "^4.2.1",
    "autoprefixer": "^10.4.16",
    "eslint": "^10.2.0",
    "eslint-config-prettier": "^10.1.8",
    "eslint-plugin-react-hooks": "^7.0.1",
    "eslint-plugin-react-refresh": "^0.4.26",
    "eslint-plugin-unused-imports": "^4.4.1",
    "globals": "^15.0.0",
    "jsdom": "^27.3.0",
    "postcss": "^8.4.32",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.9.2",
    "typescript-eslint": "^8.58.1",
    "vite": "^5.0.8",
    "vitest": "^4.1.3"
  }
}
```

Key deps to match `frontend/package.json` versions exactly (so copied Electric code compiles):
- `@tanstack/electric-db-collection`: ^0.3.12
- `@tanstack/react-db`: ^0.1.82
- `@tanstack/react-query`: ^5.96.2

After editing package.json, run `cd remote-frontend && npm install` to install.

### File: `remote-frontend/eslint.config.js` (CREATE)

**Sibling alignment:** Read `frontend/eslint.config.js`. The hive config is a SLIMMED-DOWN version: drop the i18next plugin, the check-file naming rules (hive has no naming convention enforced yet), the NiceModal restrictions (hive has no modals), and the executor-schemas plugin. Keep: js recommended, typescript-eslint recommended, react-hooks, react-refresh, unused-imports, prettier config. Justify each dropped rule in the decisions ledger.

```js
// @ts-check
import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import unusedImports from 'eslint-plugin-unused-imports';
import prettierConfig from 'eslint-config-prettier';
import globals from 'globals';

export default tseslint.config(
  { ignores: ['dist/**', 'eslint.config.js'] },
  {
    files: ['**/*.{ts,tsx}'],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: {
      globals: { ...globals.browser, ...globals.es2020 },
      parserOptions: { project: './tsconfig.json' },
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
      'unused-imports': unusedImports,
    },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      'react-refresh/only-export-components': 'off',
      'unused-imports/no-unused-imports': 'error',
      'unused-imports/no-unused-vars': ['error', { vars: 'all', varsIgnorePattern: '^_', args: 'after-used', caughtErrors: 'none' }],
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },
  prettierConfig,
  {
    files: ['*.config.{ts,js,cjs,mjs}'],
    languageOptions: { parserOptions: { project: false } },
  },
);
```

### File: `remote-frontend/vite.config.ts` (EDIT)

**Sibling alignment:** Read `frontend/vite.config.ts` — it has the `@/*` + `shared/*` aliases in `resolve.alias`. Task 201 adds those to `tsconfig.json`; the Vite config must mirror them so the dev server + build resolve the same paths. This task adds the base `resolve.alias` block (task 201 will extend it if needed — but placing it here avoids a second edit).

**Before:**
```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3002
  }
})
```

**After:**
```ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3002
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/setupTests.ts'],
    globals: true,
  },
})
```

Note: the `test` block is the Vitest config (Vite + Vitest share one config). The `@` alias is added here so imports resolve in both dev/build and test.

### File: `remote-frontend/tsconfig.json` (EDIT)

**Sibling alignment:** Read `frontend/tsconfig.json` — it has `"@/*": ["src/*"]` and `"shared/*": ["../shared/*"]`. The hive tsconfig MUST add `@/*` so the copied swarm components (task 202) and the Electric imports (task 305) resolve. Task 201 owns the `shared/*` alias (it copies `shared/types.ts`); this task adds `@/*` only.

Read the current `remote-frontend/tsconfig.json` first; add `"@/*": ["src/*"]` to the `paths` object (create `paths` if absent). Add `"types": ["vitest/globals", "@testing-library/jest-dom"]` to `compilerOptions` so test globals + matchers type-check.

### File: `remote-frontend/src/setupTests.ts` (CREATE)

```ts
import '@testing-library/jest-dom';
```

## Allowed moves
- EDIT `remote-frontend/package.json` (add scripts + deps).
- CREATE `remote-frontend/eslint.config.js` (slimmed from `frontend/eslint.config.js`).
- EDIT `remote-frontend/vite.config.ts` (add `@` alias + vitest test config).
- EDIT `remote-frontend/tsconfig.json` (add `@/*` path + test types).
- CREATE `remote-frontend/src/setupTests.ts`.
- CREATE `remote-frontend/src/toolchain.test.ts`.
- Run `cd remote-frontend && npm install` to install the new deps.

## STOP triggers
- `npm install` fails (version conflict between the hive's `react-router-dom@^7.9.5` and the node frontend's `@6.8.1` — record the conflict in the ledger and pin the hive version to match the node frontend's `^6.8.1` if the rehosted swarm components require it). HALT and reconcile.
- `@tanstack/react-db` or `@tanstack/electric-db-collection` version in `frontend/package.json` is incompatible with the hive's React 18 — HALT; record and pin compatible versions.

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npm install
cd remote-frontend && npx vitest run src/toolchain.test.ts   # passes (vitest works)
cd remote-frontend && npm run lint                             # eslint runs (0 errors on the slimmed config)
cd remote-frontend && npx tsc --noEmit                          # tsconfig changes type-check
```
All exit 0. Record the exact dep versions installed in the ledger (for traceability).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/toolchain.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 100` exits 0
