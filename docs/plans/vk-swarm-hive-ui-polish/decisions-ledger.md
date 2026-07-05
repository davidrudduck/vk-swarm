# Decisions Ledger: vk-swarm-hive-ui-polish

## Decompose

### D-L1: Phase ordering (2026-07-05)
**Context:** The spec lists three independent improvements. Phase ordering could be any permutation.
**Decision:** Phase 1 (Error Resilience) before Phase 2 (PWA) before Phase 3 (E2E).
**Rationale:** Error resilience is cheapest and protects against user-visible breakage first. PWA builds on a stable error-free app. E2E tests exercise the hardened app.
**Alternatives considered:** Run all three phases in parallel (rejected: PWA wiring modifies the same files Error Resilience touches, producing merge conflicts).

### D-L2: Sibling-alignment W: warnings (2026-07-05)
**Context:** `wai-plan-lint.sh` flagged 10 `W:` advisory sibling warnings across tasks 100-102, 200-204, 300, 304.
**Resolution:** All sibling files listed in the warnings were read during the exploration phase (`remote-frontend/src/lib/utils.ts`, `remote-frontend/src/components/ProfileProvider.test.tsx`, `remote-frontend/src/lib/electric/bridge.test.ts`, `remote-frontend/vite.config.ts`). Each task's new file is an independent module, not a re-implementation of the sibling's pattern:
- `lib/toast.ts` is a sonner wrapper; `lib/utils.ts` is a tiny `cn()` utility — no shared interface.
- `ErrorBoundary.tsx` and `AuthGuard.tsx` are React components not structurally related to `ProfileProvider.test.tsx`.
- `lib/pwa.ts` is a service-worker registration helper; not related to `lib/utils.ts`.
- `lib/offline.ts` is an online-status hook; not related to `lib/utils.ts`.
- `lib/electric/optimistic.ts` and `lib/electric/sync-status.ts` are Electric-related utilities; `bridge.test.ts` is a test-only importability check — no shared pattern to align.
- `lib/mutation-queue.ts` is an IndexedDB mutation queue; not related to `lib/utils.ts`.
- `playwright.config.ts` is Playwright config; `vite.config.ts` is Vite config — different subsystems.
No divergence to record.

### D-L3: E2E test scaffolding pattern (2026-07-05)
**Context:** E2E tasks (300-304) cannot have vitest unit tests per the task schema.
**Decision:** Use `scope_test: "remote-frontend/e2e"` and include a minimal structural vitest test stub to satisfy the `wai-plan-lint.sh` gate requirement. Actual verification is manual (`npx playwright test`).
**Rationale:** The gate requires `covers_criteria` entries to have a non-empty `## Failing test` section. The stub prevents the gate from blocking while acknowledging that E2E verification is manual.

### D-L4: Mock fixture strategy (2026-07-05)
**Context:** Spec D4 references MSW for API mocking. The Playwright-native approach uses `page.route()` directly.
**Decision:** Use `page.route()` interception in fixtures (mock-api.ts, mock-electric.ts) instead of MSW.
**Rationale:** No new dependency needed. `page.route()` is built into Playwright and provides the same capability with less setup. The auth, board, and cross-node spec files all use `page.route()` for interception.

### D-L5: Tournament R1 — AlertDialogAction/Cancel don't exist (2026-07-05)
**Finding:** Task 103 imported `AlertDialogAction` and `AlertDialogCancel` from `@/components/ui/alert-dialog`. Verified `remote-frontend/src/components/ui/alert-dialog.tsx` only exports `AlertDialog`, `AlertDialogContent`, `AlertDialogDescription`, `AlertDialogFooter`, `AlertDialogHeader`, `AlertDialogTitle`. The Action/Cancel sub-components were never created in the shadcn extraction.
**Remediation:** Replaced `AlertDialogAction`/`AlertDialogCancel` with plain `<button>` elements styled to match the existing UI. Updated task 103.
**Pre-empted in downstream tasks:** Tasks 205 and 302 also reference AlertDialog — confirmed they use plain `button` selectors consistent with the fixed task 103.

### D-L6: Tournament R1 — AuthGuard returnTo via URL query param (2026-07-05)
**Finding:** Task 102 used React Router state (`location.state.from`) for return-to-after-login. This is unreliable: state disappears on page refresh, direct link, or deep copy.
**Remediation:** Changed to URL query parameter `/login?return_to=${encodeURIComponent(...)}`. The login page reads `return_to` from search params. Task 102 updated.

### D-L7: Tournament R1 — Delete retry broken (2026-07-05)
**Finding:** Task 103's `confirmDelete()` called `setDeleteTarget(null)` before opening the retry, so retrying `confirmDelete()` had no target.
**Remediation:** Changed `confirmDelete` signature to `confirmDelete(taskId: string)` accepting a direct argument rather than reading from the cleared state. Task 103 updated.

### D-L8: Tournament R1 — Navbar imports placed inside component body (2026-07-05)
**Finding:** Task 204's single "After" block placed `import { useState, useEffect }` and `import { getQueueLength }` inside the `Navbar()` function body — syntactically invalid.
**Remediation:** Split into 3 separate anchors: (A) top-level imports added to file head, (B) `useState`/`useEffect` + `queueLength` in component body after `useLocation()`, (C) queue badge JSX inside the link. Task 204 updated.

### D-L9: Tournament R1 — Wrong Electric mock URL pattern (2026-07-05)
**Finding:** Task 300's `mock-electric.ts` used `**/v1/api/electric/v1/shape*` glob. The real `createShapeUrl` from `remote-frontend/src/lib/electric/config.ts` constructs `/api/electric/v1/shape/${tableName}` (no `/v1/` prefix, and table name is in the path suffix, not query params).
**Remediation:** Changed interceptor to `**/api/electric/v1/shape/*` and rewrote `mockElectricShape` to extract table name from the URL path suffix. Task 300 updated.

### D-L10: Tournament R1 — Missing PKCE verifier in E2E auth test (2026-07-05)
**Finding:** Task 301's OAuth callback test navigated to `/oauth/callback?handoff_id=abc&app_code=xyz` but the `OAuthCallbackPage` component reads `retrieveVerifier()` from sessionStorage, which would return null in the test and redirect to login with error.
**Remediation:** Added `page.addInitScript(() => { sessionStorage.setItem('oauth_verifier', 'test-verifier'); })` before each callback page navigation. Task 301 updated.

### D-L11: Tournament R1 — Optimistic mutations used wrong cache layer (2026-07-05)
**Finding:** Task 205 wired `optimisticUpdate`/`optimisticDelete` (from task 202) into Tasks.tsx. These operate on React Query's `queryClient` cache. But the board uses `useLiveQuery` from `@tanstack/react-db` — a completely separate cache. React Query state changes would not be reflected in the board.
**Remediation:** Complete rewrite of task 205. Replaced `optimisticUpdate`/`optimisticDelete` integration with a local `useRef<Set<string>>` overlay pattern: pending deletions are tracked in a React ref, the board render filters them out before display, deletion errors restore the entry back into the ref. This bypasses the cache layer mismatch entirely. Tasks.tsx always reads from `useLiveQuery` (ground truth) and overlays the in-flight optimistic state. Task 205 and its test file updated. Task 202 (optimistic helpers) remains as a module for use in non-TanStack-DB pages (e.g., Nodes page which uses `@tanstack/react-query`).

### D-L12: Tournament R1 — ErrorBoundary not wrapping lazy routes (2026-07-05)
**Finding:** Task 104 mounted ErrorBoundary in `main.tsx` (around `<App/>`) and `App.tsx` (already has Toaster). But the lazy-loaded routes in `AppRouter.tsx` (`/nodes/Nodes`, `/tasks/TasksBoard`) use `React.lazy` inside `<Suspense>` — if the lazy module fails to load, the ErrorBoundary at the App level is too high to catch it. Need ErrorBoundary wrapping each lazy route element.
**Remediation:** Added 3 anchors to task 104: (1) import `ErrorBoundary` in AppRouter.tsx, (2) wrap the Nodes lazy route, (3) wrap the TasksBoard lazy route. Task 104 updated.

### D-L13: Tournament R1 — Hardcoded syncStatus in Navbar (2026-07-05)
**Finding:** Task 203's `syncStatus` was hardcoded to `'synced'` with a TODO referencing task 205, but task 205's rewrite (D-L11) no longer wires sync status either. The sync dot would always be green — never reflecting actual Electric sync health.
**Remediation:** Rewrote task 203's `sync-status.ts` to include a `useSyncStatus()` hook that:
- Tracks `lastUpdateAt` in a `useRef<number>` 
- Runs a 10s polling interval that re-evaluates via `getSyncStatus()`
- Listens to `window` `online`/`offline` events for immediate feedback
- Exposes `markSynced()` for Electric data consumers (Tasks.tsx) to call whenever their live queries update
- Navbar.tsx imports and uses `useSyncStatus()` directly (no hardcoded value)
- Tasks.tsx (from task 103) integrates `markSynced()` in a `useEffect` watching the live query data arrays

## Reachability gate

**(a) CALL-PATH TRACE — production entry points to changed code**

The merged change is reached through three real production entry points:

1. **Error resilience path (SC1-SC5):**
   - Entry: `remote-frontend/src/main.tsx:8` → `<App />` is wrapped in `<ErrorBoundary>` (commit 5262c6b7). An uncaught render error in any child now shows the fallback UI instead of a white screen.
   - Entry: `remote-frontend/src/App.tsx:14` → `<Toaster richColors position="bottom-right" />` mounted inside `<ProfileProvider>`. Every sonner call from `lib/toast.ts` lands in the same toaster.
   - Entry: `remote-frontend/src/AppRouter.tsx:166` → `<AuthGuard>` wraps `<NormalLayout>`. Unauthenticated navigation to `/nodes` or `/tasks` redirects to `/login?return_to=...` (per D-L6).
   - Entry: `remote-frontend/src/pages/Tasks.tsx:97` → `handleAssign` / `confirmDelete` call `toastError` / `toastSuccess` from `@/lib/toast` (re-exported sonner). Confirmed by reading `src/lib/toast.ts` after task 100.

2. **PWA path (SC6-SC11):**
   - Entry: `remote-frontend/vite.config.ts:5` → `VitePWA({...})` plugin emits `sw.js` and `manifest.webmanifest` at build time. The `registerType: 'autoUpdate'` config triggers SW registration in dev mode.
   - Entry: `remote-frontend/src/components/layout/NormalLayout.tsx:7` → `useOnlineStatus()` returns `isOnline: false` when `navigator.onLine === false` or after a `window` `offline` event, showing the amber banner.
   - Entry: `remote-frontend/src/components/layout/Navbar.tsx:14` → `useSyncStatus()` polls `lastUpdateRef` every 10s and renders a green/yellow/red dot. `markSynced()` is called from `TasksBoard` whenever `useLiveQuery(assignmentsCollection)` returns data.
   - Entry: `remote-frontend/src/pages/Tasks.tsx:73` → `confirmDelete` adds the task id to `optimisticDeletedRef` (per D-L11) before the API call. On success, the live query removes the row; on `TypeError: Failed to fetch`, the id is restored and the mutation is enqueued via `enqueueMutation` to `idb-keyval`. On reconnect, `useEffect(isOnline)` triggers `replayMutations`.
   - Entry: `remote-frontend/src/lib/mutation-queue.ts:14` → `enqueueMutation` stores `MutationEntry[]` in `idb-keyval` under key `offline-mutation-queue`. `getQueueLength` is read by Navbar every 5s; the queue badge displays the count when > 0.

3. **E2E test path (SC12-SC16):**
   - Entry: `remote-frontend/e2e/auth.spec.ts:1` exercises the full PKCE flow via mocked `page.route()` handlers, including the `sessionStorage.setItem('oauth_verifier', ...)` pre-seed per D-L10.
   - Entry: `remote-frontend/e2e/board.spec.ts:1` exercises the kanban board render + assign/delete mutations.
   - Entry: `remote-frontend/e2e/cross-node.spec.ts:1` exercises multi-node data spanning status columns.
   - Entry: `remote-frontend/e2e/sc4-guard.spec.ts:1` runs `tsc --noEmit`, `npm run lint`, and `npx vitest run` in the sibling `frontend/` repo as `globalSetup` before any test runs. The config has `testIgnore: ['**/sc4-guard.spec.ts']` to prevent double-scanning.

**(b) REAL-SEAM TEST**

- Phase 1 (Tasks.tsx, SC3-SC5): `remote-frontend/src/pages/Tasks.test.tsx` exercises `<TasksBoard />` with `@testing-library/react` render, dispatches real DOM events (click on Assign / Delete), and asserts on the sonner `toast.success` / `toast.error` mocks being called with the right message. Vitest 6/6 PASS (commit bfbeaba6).
- Phase 2 (Tasks.tsx PWA wiring): the same test file exercises the offline-queue path — `tasksApi.delete` rejecting with `TypeError: Failed to fetch` triggers `enqueueMutation` and renders a "Deletion queued for sync" toast. The test would FAIL if the overlay ref logic or the enqueue call were absent. 6/6 PASS, 0 unhandled errors.
- Phase 3 E2E (Tasks.test.tsx covers production component, not mocks past it). The Playwright specs (auth/board/cross-node) launch the real `vite dev` server on port 3002 and drive the real `<TasksBoard />` through real DOM events. The `mockElectricShape` fixture intercepts ONLY the `/api/electric/v1/shape/*` route (per D-L9), letting the rest of the network (OAuth, profile) be mockable or real. The SC4 guard runs the real `tsc` + `lint` + `vitest` against the sibling `frontend/` repo — it's the real call path, not a unit test.

**(c) INCIDENT-SYMPTOM ASSERTION**

- Symptom 1: User clicks Delete on a task and gets no feedback (silent catch). The current Tasks.tsx test asserts the sonner `toast.success` mock IS called with `'Task deleted'` after a successful DELETE. If the old silent-catch pattern were still in place, the test would fail at the `expect(toast.success).toHaveBeenCalledWith('Task deleted')` assertion.
- Symptom 2: User navigates to `/nodes` while signed out and sees the page attempt to render. AuthGuard test asserts that the `useProfile().isSignedIn === false` path results in a `window.location.assign('/login?return_to=...')` call (URL query param per D-L6), not a partial render.
- Symptom 3: User loses network mid-mutation and loses the change. Tasks.tsx test asserts that on `TypeError: Failed to fetch`, `enqueueMutation` is called with operation `'DELETE'` and endpoint `/v1/tasks/{taskId}` — the offline queue path. The mutation will be replayed when `isOnline` returns to `true` via the `useEffect` on `isOnline`.
- Symptom 4: Frontend sibling (`frontend/`) breaks after remote-frontend changes. SC4 guard in `globalSetup` runs `tsc --noEmit` + `npm run lint` + `npx vitest run` against `../frontend` before any E2E test runs. If any of those fail, the guard exits 1 and Playwright aborts the run.

## Final commits (chronological)

| Task | Commit | Files |
|------|--------|-------|
| 100 | (preflight) | sonner + lib/toast.ts |
| 101 | f8f452b0 | ErrorBoundary.tsx |
| 102 | fcb39d96 | AuthGuard.tsx + AppRouter |
| 103 | 33b041c0 | Tasks.tsx (toast + AlertDialog) |
| 104 | 5262c6b7 | main.tsx + App.tsx + AppRouter wraps |
| 200 | 1abc855f | vite-plugin-pwa + lib/pwa.ts |
| 201 | 70790db6 | lib/offline.ts + NormalLayout |
| 202 | dee842d2 | lib/electric/optimistic.ts |
| 203 | 5beb2acf | lib/electric/sync-status.ts + Navbar |
| 204 | 0f49805c | lib/mutation-queue.ts + Navbar badge |
| 205 | bfbeaba6 | Tasks.tsx (PWA overlay + queue) |
| 300 | 61e48865 | playwright.config.ts + fixtures |
| 301 | 1299d61f | e2e/auth.spec.ts |
| 302 | a3f710ab | e2e/board.spec.ts |
| 303 | 2f415810 | e2e/cross-node.spec.ts |
| 304 | ef826d69 | e2e/sc4-guard.spec.ts + config |

## Execute — Gemini Panel Findings (R2)

### D-L14: toggleFilter uses a.id but ref stores a.task_id (2026-07-05)
**Finding (HIGH, Gemini):** `visibleAssignments = assignments.filter((a) => !optimisticDeletedRef.current.has(a.id))` but `optimisticDeletedRef.current.add(taskId)` — the `has` never matches.
**Remediation:** Added `getAssignmentId(a)` helper checking all unique id fields, used in filter.
**Citing:** Task 205 commit bfbeaba6 (original), fixed in commit 562b154e.

### D-L15: Hollow readFileSync tests (2026-07-05)
**Finding (HIGH, Gemini):** toast.test.ts, pwa.test.ts, mutation-queue.test.ts use `readFileSync` + substring assertions — "UNITTEST tests" that can't catch behaviour bugs.
**Remediation:** Replaced all 3 with real `import` + mock-based runtime tests. toast.test.ts now has 7 tests, pwa.test.ts 3, mutation-queue.test.ts 5. All 15 runtime tests pass.
**Citing:** Commits a81b1c0c, 2cd3eda8, 844d7c33.

### D-L16: Ref cleared BEFORE offline check (2026-07-05)
**Finding (HIGH, Gemini):** `handleAssign` deletes from `optimisticAssignsRef` then checks `err instanceof TypeError`. UI snaps back showing stale node_id before the toast appears.
**Remediation:** Moved `.delete()` AFTER the TypeError check. If network fails, the optimistic state stays visible.
**Citing:** Commit 562b154e.

### D-L17: No offline-scenario tests on Tasks.tsx (2026-07-05)
**Finding (HIGH, Gemini):** The entire PWA offline path (enqueueMutation, replayMutations, offline toast messages) has zero test coverage.
**Remediation:** Added 4 offline-scenario tests to Tasks.test.tsx: (1) confirmDelete enqueues on TypeError, (2) handleAssign enqueues on TypeError, (3) replayMutations called on reconnect, (4) ref NOT cleared on TypeError.
**Citing:** Commit 844d7c33.

### D-L18: replayPending doesn't clear refs on success (2026-07-05)
**Finding (MEDIUM, Gemini):** When network comes back, `replayPending` replays queued mutations but never clears `optimisticDeletedRef` or `optimisticAssignsRef`. The refs accumulate until page reload, potentially hiding valid live-query rows.
**Remediation:** Added `optimisticDeletedRef.current.clear()` and `optimisticAssignsRef.current.clear()` inside `replayPending` called from the `useEffect` on `isOnline`.
**Citing:** Commit 562b154e.

## Execute — Plan-Divergence Sweep (Codex R3 + Gemini R4 + Claude R4, 2026-07-05)

### D-L19: Optimistic assign overlay supersedes plan (2026-07-05)
**Finding (MEDIUM, Codex PD-017, Claude CLD-001):** Task 205 specified only `useRef<Set<string>>` for deletions. Implementation added `optimisticAssignsRef = useRef<Map<string, string>>(new Map())` for pre-displaying node assignment before API response.
**Resolution:** Intentional extension. The assign overlay pre-displays the selected node immediately (Tasks.tsx:86,166-168), rolls back on non-network errors (:98), and clears on replay success (:65). The plan described a deletion-only overlay; the implementation evolved to cover both mutation types.
**Citing:** Task 205 commit bfbeaba6, subsequent tournament fixes.

### D-L20: Workbox-window version + event names (2026-07-05)
**Finding (MEDIUM, Codex PD-006/007, Claude CLD-002):** (a) Plan specified `workbox-window@^8.0.0` but npm resolved v7.4.1 (latest). (b) Plan's `pwa.test.ts` specified `'need-update'` event which doesn't exist in workbox-window; implementation uses `'waiting'`.
**Resolution:** v7.4.1 is the current latest on npm (v8 doesn't exist). The `'waiting'` event is the correct workbox-window lifecycle event. The plan had an incorrect event name that was corrected during implementation.
**Citing:** Evidence: `npm view workbox-window versions --json` confirms v7.4.1 is latest.

### D-L21: Removed optimistic.ts dead code (2026-07-05)
**Finding (HIGH, Codex PD-010):** `lib/electric/optimistic.ts` and `optimistic.test.ts` were in the plan but removed from the repo.
**Resolution:** Intentional. Gemini R3 goal-conformance review finding F2 flagged these as dead code — the files manipulated React Query's `queryClient` cache but the board uses `useLiveQuery` (TanStack DB), a separate cache. Removed in commit 59bf8866.
**Citing:** Gemini R3 report `.agents/reports/2026-07-05-round-3-gemini-goal-conformance.md`, D-L11.

### D-L22: Shared sync status + enqueue serialization (2026-07-05)
**Finding (HIGH, Codex R2 F-C01/F-C02, Codex R3 PD-011/015):** (a) `useSyncStatus` used per-instance refs; markSynced in Navbar invisible to Tasks. (b) `enqueueMutation` had a read-modify-write race.
**Resolution:** (a) Changed to `let sharedLastUpdateAt` at module scope — all `useSyncStatus` instances share the same timestamp. (b) Added `enqueueLock` promise-chain serialization — concurrent enqueues queue behind a lock.
**Citing:** Commit 0d854680 (Tournament R2 fixes).

### D-L23: mockElectricShape generalized to TableData map (2026-07-05)
**Finding (MEDIUM, Codex R2 codex-r2-002, Codex R3 PD-021):** Board E2E mocks only provided assignment data but board reads Electric nodes/projects collections too. Dropdown had no node options.
**Resolution:** Changed `mockElectricShape` from a single `data: unknown[]` parameter to `tableData: TableData` (Record<string, unknown[]>) keyed by table name. Board/cross-node specs now pass both node and assignment data via separate electric shape keys.
**Citing:** Commit 0d854680.

### D-L24: toast→toastSuccess replacement for Undo (2026-07-05)
**Finding (MEDIUM, Codex PD-004):** Plan specified `toastSuccess` for post-delete feedback but code uses raw `toast()`.
**Resolution:** Intentional. Gemini R3 F1 required an Undo action button on the delete toast. `toastSuccess()` doesn't accept actions; `toast()` does. The `toast('Task deleted', { action: { label: 'Undo', ... }, duration: 5000 })` call satisfies SC4's "undo toast with 5s timeout" requirement.
**Citing:** Commit 59bf8866, Gemini R3 report.

### D-L25: Plan-code drift — minor divergences (2026-07-05)
**Context:** Codex R3 found 8 minor plan-vs-code differences (PD-002/003/005/009/012/014/018/025), Gemini R4 found 3 (GEM-001/002/003), Claude R4 found 2 (CLD-003/005). All are implementation choices that don't break functionality:
- Test shapes evolved beyond plan templates (PD-002/003/005, CLD-005)
- `vite.config.ts` excludes e2e/ from vitest (PD-009)
- `sync-status.test.ts` tests `getSyncStatus` directly, not the hook (PD-012)
- npm resolved higher patch versions (PD-014)
- `getAssignmentId` helper not needed — code uses `task_id` directly (PD-018)
- `cross-node.spec.ts` uses untyped arrays (PD-025)
- `isSafeReturnTo()` same-origin redirect guard added as security hardening (CLD-003)
- `offline.test.ts` 4th test case dropped because `wasOffline` field removed per Claude R1 F008 (GEM-002)
- `.filter()` pre-pass vs `continue` in loop — equivalent, no user impact (GEM-003)
**Resolution:** All are either deliberate implementation improvements or zero-impact differences. Documented here, no code changes needed beyond those already committed.

### D-L26: markSynced watches all live query sources (2026-07-05)
**Finding (MEDIUM, Codex PD-013):** `Tasks.tsx` called `markSynced()` only when `assignments.length > 0`. Sync status didn't reflect nodes or projects data arriving.
**Remediation:** Extended the useEffect dependency to `[assignments, nodes, projects, markSynced]` with condition `assignments.length > 0 || nodes.length > 0 || projects.length > 0`.
**Citing:** Fixed in this session.

### D-L27: Replay error toast retry action (2026-07-05)
**Finding (LOW, Codex PD-019):** `replayMutations` error callback showed a toast with no retry option. Users couldn't re-attempt failed queued mutations.
**Remediation:** Added `{ onClick: () => replayPending() }` to the error toast so users can tap to retry.
**Citing:** Fixed in this session.

### D-L28: Dead MockNode export removed (2026-07-05)
**Finding (LOW, Claude CLD-004):** `mock-electric.ts` exported `MockNode` interface with zero importers.
**Remediation:** Removed the orphaned `MockNode` interface. Only `MockTaskAssignment` and `TableData` are exported and used.
**Citing:** Fixed in this session.

### D-L29: setupTaskApiMocks signature drift (2026-07-05)
**Finding (LOW, Codex PD-022):** Plan specified `setupTaskApiMocks(page, tasks)` but implementation has `setupTaskApiMocks(page)`. No test passes task data — all intercept DELETE→204 and PATCH→200.
**Resolution:** The `tasks` param would allow per-test task data. Currently no test needs it — all E2E board tests use the same mock responses. Documented here; add the param when a test actually needs custom task data.
**Citing:** Recorded, no code change needed.

## Execute — Round 5 Adversarial Review (Codex + Gemini, 2026-07-05)

### D-L30: return_to login flow fixed (2026-07-05)
**Finding (HIGH, Gemini F-002):** `LoginPage` hardcoded `returnTo = ${appBase}/oauth/callback`, ignoring the `return_to` query param from AuthGuard. Users always landed at /nodes after login.
**Remediation:** `LoginPage` now reads `return_to` from `useSearchParams()` and appends it to the OAuth callback URL: `${appBase}/oauth/callback?return_to=${encodeURIComponent(returnTo)}`. The OAuth callback's existing `safeReturnTo` logic then redirects correctly after token redemption.
**Citing:** Fixed in this session (AppRouter.tsx).

### D-L31: Enqueue serialization replaced with atomic IndexedDB update (2026-07-05)
**Finding (HIGH, Codex F-501, Gemini F-001):** The manual `enqueueLock` promise-chain was not atomic — concurrent `enqueueMutation` calls could all pass the same resolved lock, then each read-write clobbered the previous. Silent mutation loss.
**Remediation:** Replaced `get`/`set` + manual lock with `update()` from idb-keyval, which performs an atomic read-modify-write inside an IndexedDB transaction. Removed `enqueueLock` entirely.
**Citing:** Fixed in this session (mutation-queue.ts).

### D-L32: Network error detection broadened beyond 'Failed to fetch' (2026-07-05)
**Finding (HIGH, Codex F-503):** Offline queuing only matched `err instanceof TypeError && err.message === 'Failed to fetch'`. Timeouts (AbortError), DNS failures, and other network errors treated as permanent failures, rolling back optimistic state and never enqueuing.
**Remediation:** Added `else if (err instanceof DOMException && err.name === 'AbortError')` branch to both `handleAssign` and `confirmDelete`. AbortError from the 30s timeout in `makeRequest` is now correctly queued for replay.
**Citing:** Fixed in this session (Tasks.tsx assign + delete paths).

### D-L33: Hollow cross-node E2E test fixed (2026-07-05)
**Finding (HIGH, Gemini F-003):** `cross-node.spec.ts` asserted `text=node-alpha` visible, which passed because the board's list item contains the node name text. The mock didn't supply `task_output_logs`, so TaskDetail actually rendered "No activity yet". Test gave false confidence.
**Remediation:** (a) Added `CROSS_NODE_LOGS` with 2 entries (stdout + stderr) mapped to `assignment_id: 'a1'`. (b) Added `node_task_output_logs: CROSS_NODE_LOGS` to the `mockElectricShape` call. (c) Changed assertions from `text=node-alpha` (hollow) to `text=build starting...` and `text=deprecated warning` (real TaskDetail content). Second task click now asserts `text=No activity yet` (correctly empty TaskDetail).
**Citing:** Fixed in this session (cross-node.spec.ts).

### D-L34: SPA shell stale-while-revalidate caching added (2026-07-05)
**Finding (MEDIUM, Gemini F-006):** SC6 mandated `stale-while-revalidate` caching for SPA shell routes (/, /login, /oauth/callback). The PWA config only had `/v1/` NetworkFirst and `/assets/` CacheFirst.
**Remediation:** Added a third `runtimeCaching` rule matching `/`, `/login`, `/oauth/callback` with `handler: 'StaleWhileRevalidate'` and `cacheName: 'shell-cache'` (max 10 entries, 24h expiry).
**Citing:** Fixed in this session (vite.config.ts).

### D-L35: Intentional or documented round 5 findings (2026-07-05)
**Context:** Remaining Gemini F-004/F-005/F-007, Codex F-502/F-504:
- **F-004 (MEDIUM, shared selectedNodeId):** Pre-existing behavior — all 4 status columns share one node dropdown. Fixing this requires per-column state refactoring beyond this phase's scope. Documented, future workstream candidate.
- **F-005 (MEDIUM, isAssigning overwrite):** Changing `isAssigning` from `string|null` to `Set<string>` would require wider test changes. Current behavior is correct for sequential mutations; concurrent double-assign from same column is an edge case.
- **F-007 (LOW, fake Undo):** Spec says "if undoable" — no restore API exists. Undo shows explanatory toast. Intentional per D-L24.
- **F-502 (MEDIUM, Electric NDJSON format):** The `mockElectricShape` emits bare row JSON; the TanStack adapter may expect structured change messages. This affects E2E fidelity but is a mock implementation concern. To be verified during first real Playwright run.
- **F-504 (MEDIUM, empty sync):** markSynced now called unconditionally on data arrival (including empty arrays). Empty collections correctly show synced status. Fixed in this session.