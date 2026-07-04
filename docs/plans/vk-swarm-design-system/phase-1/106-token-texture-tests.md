---
id: "106"
phase: 1
title: Token + texture integration smoke (npm run build + tsc)
status: ready
depends_on: ["105"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/styles/tokens/smoke.test.ts
irreversible: false
scope_test: "remote-frontend/src/styles/tokens/smoke.test.ts"
allowed_change: create
covers_criteria: [SC2, SC3]
---

## Failing test (write first)

Create `remote-frontend/src/styles/tokens/smoke.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { execSync } from 'node:child_process';

describe('phase 1 integration smoke (SC2/SC3)', () => {
  it('npm run build exits 0 (tokens + textures compile through Vite)', () => {
    expect(() => execSync('npm run build', { stdio: 'pipe', timeout: 120_000 })).not.toThrow();
  });

  it('tsc --noEmit exits 0 (token test files type-check)', () => {
    expect(() => execSync('npx tsc --noEmit', { stdio: 'pipe', timeout: 120_000 })).not.toThrow();
  });

  it('eslint exits 0 on the tokens dir', () => {
    expect(() => execSync('npx eslint src/styles/tokens --max-warnings 0', { stdio: 'pipe', timeout: 60_000 })).not.toThrow();
  });
});
```

## Change

### File: `remote-frontend/src/styles/tokens/smoke.test.ts` (CREATE)
Create exactly as written above. The test exercises the three supplemental gate commands from `plan.md`'s Gate section: `npm run build` (Vite compiles the CSS through PostCSS + token @imports resolve), `tsc --noEmit` (token test files type-check), and `eslint` on the tokens dir (lint cleanliness). This is the phase-1 integration smoke that proves the token layer is wired end-to-end.

NOTE: no `cwd` is passed to `execSync` because vitest already runs from `remote-frontend/` (the `WAI_TEST_CMD` `cd`s there); the spawned `npm run build` / `npx tsc --noEmit` / `npx eslint` inherit vitest's cwd (`remote-frontend/`) where `package.json` / `tsconfig.json` live. Do NOT add `cwd: '..'` (would resolve to the worktree root which has no package.json build script).

## Allowed moves

- Create `remote-frontend/src/styles/tokens/smoke.test.ts` exactly as written above.
- No other file may be touched. Do NOT edit `frontend/` (SC9).

## STOP triggers

- `npm run build` fails (would mean a token `@import` path is wrong or a CSS syntax error escaped → STOP, fix the upstream task, re-run).
- `tsc --noEmit` fails (would mean a token test file has a type error → STOP, fix the upstream task).
- `eslint` fails (would mean a token test file has a lint error → STOP, fix the upstream task).

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/styles/tokens/smoke.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-design-system 106` exits 0.