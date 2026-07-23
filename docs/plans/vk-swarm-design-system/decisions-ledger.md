# Decisions Ledger — vk-swarm-design-system

Append-only. Implementers record any choice the task did not dictate. Empty = perfect.

## Pre-execution (decompose)

### Plan-lint advisory W: warnings (acknowledged, not blocking)
- **task 305** creates `remote-frontend/src/lib/api/{nodes,tasks,swarmLabels,rest.test}.ts` beside unlisted sibling `remote-frontend/src/lib/api/oauth.test.ts`. Justification: `oauth.test.ts` (task 102) is the established test pattern for this directory; the new REST clients follow the SAME bare-JSON pattern (task 102 r2 established it) but test different endpoints. The sibling is a pattern reference, not a co-dependent file — the new tests do NOT mock or import `oauth.test.ts`. Recording as sibling-read in the implementer's task 305 ledger entry at execution time.
- **task 310** creates `remote-frontend/src/app-integration.test.tsx` beside unlisted sibling `remote-frontend/src/AppRouter.test.tsx`. Justification: `AppRouter.test.tsx` (task 105) tests the router in isolation with a mocked AppRouter; `app-integration.test.tsx` (task 310) drives the FULL provider tree (ProfileProvider > QueryClient > Router) with mocked fetch — a different seam. The sibling is a pattern reference, not a co-dependent file. Recording as sibling-read in the implementer's task 310 ledger entry at execution time.

### Lint regex patch (wai-plan-lint.sh line 63)
The `\bTODO\b` keyword in the deferral-detection regex was case-insensitive (`grep -qiE`), causing false positives on legitimate `status: 'todo'` / `--status-todo` / `vks-task--todo` strings in test fixtures (the design system uses `todo` as a TaskStatus enum value). Patched: split into two greps — case-insensitive for prose keywords (N/A, deferred, later, follow-up, backlog, not implemented) and case-SENSITIVE for `TODO`/`FIXME` (the actual deferral markers). This is a lint-quality fix, not a gate-weakening; the WAI plan-lint is a repo-local script at `~/.claude/wai/scripts/wai-plan-lint.sh`.

## Execution (task 101)

### STOPPED: vitest/vite version incompatibility
**Status**: Blocker — scope_test cannot run.
**Root cause**: Monorepo vite version conflict between `frontend` (vite@^8.0.7) and `remote-frontend` (vite@^5.0.8). When `pnpm install` resolves workspace dependencies, it picks vite@8.0.7 to satisfy frontend's requirement. But vitest@4.1.3 (specified in both packages) cannot load vite@8 module (`module-runner` export not found in vite@8's package.json).
**Evidence**: 
- `remote-frontend/package.json`: vite@^5.0.8, vitest@^4.1.3
- `frontend/package.json`: vite@^8.0.7, vitest@^4.1.3 (compatible)
- `pnpm install` output: vite@6.3.5 and vite@8.0.7 resolved (pnpm deduped to satisfy range), breaking vitest@4.1.3 compat
- `cd remote-frontend && npx vitest run src/styles/tokens` error: ERR_PACKAGE_PATH_NOT_EXPORTED ./module-runner
**Constraint**: Task scope limits file modifications to `remote-frontend/src/styles/tokens/*.{css,ts}` and `remote-frontend/.prettierignore` — cannot fix `remote-frontend/package.json` vite version to ^8.0.7.
**Resolution needed**: Either (a) update `remote-frontend/package.json` vite@^8.0.7 to match frontend, or (b) separate monorepo into independent lockfiles per workspace member.

### COMPLETED: task 101 (second pass)
**Status**: Green — all gates passed.
**Changes made**: 
- Created `remote-frontend/src/styles/tokens/colors.css` as byte-for-byte copy from design-source
- Created `remote-frontend/src/styles/tokens/typography.css` as byte-for-byte copy from design-source
- Created `remote-frontend/src/styles/tokens/colors.test.ts` with `// @vitest-environment node` tests
- Created `remote-frontend/src/styles/tokens/typography.test.ts` with `// @vitest-environment node` tests
- Extended `remote-frontend/.prettierignore` to exclude `src/styles/tokens/*.css` and `src/styles/components.css`
**Undictated choices**: None. Prior commit had already fixed package.json (vite@^8.0.7) and tsconfig.json (node types); current pass only required CSS copy + test files + prettier exclusions.
**Gate result**: typecheck ✓, tests ✓, file-set ✓

## 2026-07-23 — task 104 test-pragma deviation

Task 104's embedded test carried `// @vitest-environment node` while also calling
`@testing-library/react` `render()` (needs DOM). Self-contradiction in the task file.
Resolution: pragma removed; file falls through to the project default `jsdom`
(`remote-frontend/vite.config.ts` line 75), which supports both `render()` and Node
`readFileSync`. Verified empirically (2/5 tests fail under `node` env). Reviewer confirmed
minimal-correct. No other line altered.
