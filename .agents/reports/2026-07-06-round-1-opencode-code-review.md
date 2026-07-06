# Code Review — Round 1

**Target:** branch `opencode/proud-panda`   **Range:** `fe52c507..HEAD`   **Effort:** high

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `remote-frontend/src/lib/mutation-queue.ts:30-48` | medium | correctness | `replayMutations` reads the queue via `get`, processes entries, then calls `set(remaining)`. A concurrent `enqueueMutation` (uses atomic `update`) can append new entries between the `get` and `set`. The `set` overwrites the entire queue, silently dropping mutations enqueued during replay. React strict mode double-mount makes this especially likely in dev. | high | yes |
| 2 | `remote-frontend/src/pages/Tasks.tsx:91-95,133-137` | medium | correctness | `await enqueueMutation(...)` inside catch blocks has no inner try/catch. If IndexedDB fails (quota, transaction conflict), the error propagates out as an unhandled promise rejection — no error toast, silent data loss since the API also already failed. | medium | yes |
| 3 | `remote-frontend/src/components/AuthGuard.test.tsx:47-48` | low | quality | Test "redirects to /login when signed out" only asserts protected content and loading spinner are absent. Never verifies the actual redirect target (e.g., URL change to `/login`). A false `<></>` fragment would also pass. | high | yes |
| 4 | `remote-frontend/src/pages/Tasks.test.tsx:111,152` | low | quality | `getAllByText('Delete').pop()!` and `deleteButtons[deleteButtons.length - 1]` rely on task-card button preceding dialog button in DOM. If the dialog uses a portal, `pop()` returns `undefined` (silent crash via `fireEvent.click(undefined)`) or the wrong element. | high | yes |
| 5 | `remote-frontend/src/pages/Tasks.tsx:125-130` | low | quality | Success toast after `confirmDelete` shows an "Undo" action button. Clicking it shows "Undo not available for this task" — a dead-end that misleads the user. | high | yes |
| 6 | `docs/development/remote-frontend.mdx:92` | low | quality | Doc claims ErrorBoundary is mounted in `main.tsx`, `App.tsx`, and `AppRouter.tsx`. `App.tsx` contains zero ErrorBoundary imports or JSX — only 2 mount points exist. | high | yes |
| 7 | `docs/development/remote-frontend.mdx:178-179` | low | quality | Doc describes `sc4-guard.spec.ts` as "verifies the dev server is responding before any test runs." The actual code runs `tsc --noEmit`, `npm run lint`, and `npx vitest run` in the sibling `frontend/` repo — a full CI gate, not a health-check. | high | yes |
| 8 | `remote-frontend/e2e/auth.spec.ts:84-89` | low | quality | Test "logout clears token and redirects to /login" only asserts `expect(token).toBeNull()` — never verifies the redirect via `waitForURL` or URL assertion. Test name promises more than it verifies. | high | yes |
| 9 | `remote-frontend/e2e/board.spec.ts:5` | low | quality | `MockTaskAssignment[]` type annotation used at line 5 but never imported. With `isolatedModules: true`, this is a TypeScript error for any tool that type-checks the e2e directory. Playwright's esbuild transpiler strips types, so it passes at runtime but fails `tsc`. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| N-1 | `remote-frontend/src/lib/pwa.ts:10-17` + `vite.config.ts:10` | low | quality | Custom Workbox `waiting`/`activated` listeners set `showRefreshPrompt` and call `window.location.reload()`. With `registerType: 'autoUpdate'`, vite-plugin-pwa injects its own skipWaiting+reload. The custom handlers are redundant — `autoUpdate` already handles the full update flow. | medium | Pre-existing deliberate choice (vite-plugin-pwa v7); custom handlers are dead code but harmless. |
| N-2 | `remote-frontend/e2e/sc4-guard.spec.ts:18,26,34` | low | quality | Three `catch` blocks call `process.exit(1)`. A thrown `Error` would let Playwright report the failure, but `process.exit(1)` is the standard CLI exit pattern for guard scripts. | high | Intentional — CLI guard scripts use `process.exit(1)` for explicit non-zero exit codes. |
| N-3 | `remote-frontend/e2e/cross-node.spec.ts:60` + `e2e/fixtures/mock-electric.ts:25` | low | quality | Files are missing a trailing newline (POSIX violation). | high | Cosmetic. Editor config / CI linter will normalize this. |

## Verdict: Request changes

Actionable: [1, 2, 3, 4, 5, 6, 7, 8, 9]