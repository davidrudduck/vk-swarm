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

## 2026-07-23 — task 106 test-timeout deviation

Task 106's embedded smoke tests set execSync timeouts (120s/120s/60s) but no vitest per-test
timeout; vitest's 5s default would fail every test deterministically. Resolution: added the
matching timeout as third arg to each it(). No assertion or command changed. Reviewer
confirmed minimal-correct.

## 2026-07-23 — phase 1 integrated adversarial review (round 1)

Panelists: Codex + OpenCode (agy/Gemini quota-exhausted; OpenCode substituted). Reports at
`.agents/reports/2026-07-23-round-1-{codex,opencode}-phase1-tokens.md`. Both FIX-FIRST.

Fixed in-session (commit 4bf7d617):
- F1/F2/F3 Preflight cascade — index.css restructured to Tailwind v3 @import form; built-CSS
  byte offsets confirm base.css rules now follow Preflight. Token files stayed byte-identical.
- F4 NodeCard hsl(hex) → var() (closes backlog F-2026-07-22-01).
- F5/F6 base.test.ts wrong-token assertion + fragile selector check.

Accepted, not fixed (rationale):
- Google Fonts external @import (Codex/OpenCode F8): plan-frozen (spec sha cd78aed7); --font-ui
  fallback chain degrades gracefully offline. Revisit if PWA offline fidelity becomes a criterion.
- Nested dark-theme inheritance (Codex #3): verbatim design-source behavior; byte-identity
  constraint governs; no nested-theme usage exists in remote-frontend.
- F7 vks-pulse keyframe: arrives with components.css in task 201 (phase 2, this branch).
- F9 tailwind.config theme mapping: pre-existing; phase-3 tasks 307/310 own shell integration
  and will surface it if shadcn utilities are actually relied on.

## 2026-07-23 — phase 2 execution notes

- Tasks 202-208: recurring strict-TS fixes to plan-literal test snippets (querySelector null
  handling, unused imports, TS2430 Omit<...,'title'>) — all declared, all reviewer-adjudicated
  minimal-correct. JSX held authoritative over task prose where they disagreed (205 title <p>,
  206 offline-pulse BEM modifier).
- Task 208: index.css anchor prose was stale (predated the round-1 remediation restructure).
  components.css wired after 'tailwindcss/components', before 'tailwindcss/utilities';
  cascade property (.vks-* after Preflight) preserved and regression-tested in
  tokens/index.test.ts (additive out-of-files-list edit, orchestrator-authorized).

## 2026-07-23 — phase 2 integrated adversarial review (round 2)

Panelists: Codex + OpenCode. Reports: `.agents/reports/2026-07-23-round-2-{codex,opencode}-phase2-components.md`. Both FIX-FIRST.

Fixed in-session (commit 1913d1c3):
- Tabs WAI-ARIA keyboard pattern (roving tabIndex, arrows/Home/End) — additive beyond
  design-source JSX anatomy; classes/DOM unchanged; +tests.
- Switch/Checkbox extend Omit<ButtonHTMLAttributes,...> — SettingsRow htmlFor now composes.
- StatusBadge `?? status` fallback restored (JSX parity); Badge type-only import.

False positive (documented, no change):
- OpenCode C1 "unlayered components.css beats @layer utilities": Tailwind v3.4 emits plain
  unlayered CSS (native @layer is v4-only). Built CSS: 0 `@layer`; utilities land after
  .vks-badge (byte offsets 23170/26052 vs 12443) → source order lets utilities win.
  Byte-identity of components.css preserved.

Accepted/no-change:
- I1 vks-pulse keyframes now animate swarm/NodeCard — intended; closes round-1 F7.
- M2 controlled onCheckedChange fires on same-value click — JSX parity.
- Codex#3 smoke-only parity test — real assertions live in per-component test files.
