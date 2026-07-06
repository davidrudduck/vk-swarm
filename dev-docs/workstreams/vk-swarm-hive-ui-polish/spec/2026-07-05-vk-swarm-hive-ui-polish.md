---
doc_type: spec
status: shipped
workstream: vk-swarm-hive-ui-polish
change_kind: behaviour
---

# vk-swarm-hive-ui-polish — Error Resilience + Offline-First PWA + E2E Test Suite

> **Parent workstream:** `vk-swarm-hive-ui` (shipped — PR #456, 234 files, 20 tasks, reachability
> gate PASS). The hive-hosted central-management console is built; this workstream hardens it from
> functional prototype to production-quality.
>
> **Audit basis:** adversarial audit of the completed `vk-swarm-hive-ui` implementation (14 quality
> categories, 50+ findings). Three high-impact gaps selected from that audit: Error Resilience
> Layer, Offline-First PWA, and Playwright E2E Test Suite.

## Intent (what / why)

The `vk-swarm-hive-ui` workstream delivered a working hive-hosted management console: OAuth PKCE auth
shell, rehosted swarm components, cross-node task kanban, and TaskDetail panel — all passing 15 Vitest
tests, the Stage-1 CONFORMS gate, and the Stage-2 adversarial panel.

But it's a prototype, not production-ready. Three gaps were identified through adversarial audit:

1. **No error resilience.** Any render crash = white screen. Signed-out users navigate to `/nodes` or
   `/tasks` and see broken pages. All task assign/delete failures silently swallowed (`catch {}`).
   No confirmation dialogs for destructive actions.
2. **No offline support.** No service worker. When Electric sync drops, `useLiveQuery` silently
   returns stale data. Task mutations go through REST with no optimistic update — user waits for
   round-trip with zero UI feedback. No reconnection indicator.
3. **Zero E2E tests.** 15 Vitest unit tests with mocked collections and fetch — they test component
   rendering, not real integration. No test exercises the full auth flow. No test verifies the
   kanban board with real data across nodes. No test confirms SC4 (node frontend still compiles).

These three improvements are:
- **Highest impact + cheapest to fix** (Error Resilience) — 3+ code-level gaps that would cause
  user-visible breakage in production on day 1.
- **Highest differentiation** (Offline-First PWA) — no management console in this class has
  optimistic mutations + offline replay.
- **Highest quality gate** (E2E Test Suite) — zero integration coverage prevents the 3-phase
  composition from regressing.

## Users / who is affected

- **Operator/admin (browser):** manages nodes, tasks, and the cross-node board. Before this
  workstream: white-screen crashes, silent mutation failures, and broken pages when signed out.
  After: ErrorBoundary fallbacks, auth guards with redirect, toast feedback on every action,
  confirmation dialogs on destructive actions.
- **End-user / coder (browser):** views and interacts with tasks/attempts/executions. Before:
  stale data during network drops, no feedback during mutations, no offline awareness. After:
  optimistic board updates, offline reconnect banner, sync status indicator, offline mutation
  queue with auto-replay.
- **Developers / CI:** before: zero integration tests, SC4 guard not enforced. After: 7+ Playwright
  E2E test cases across 4 spec files, SC4 regression guard in globalSetup.

## User stories

- **US1:** As an operator, when any component crashes, I see a styled fallback with a "Reload"
  button — not a white screen.
- **US2:** As an operator, when I'm signed out and navigate directly to `/nodes` or `/tasks`,
  I'm redirected to `/login` instead of seeing a broken page.
- **US3:** As an operator, when I assign or delete a task and it fails, I see an error toast
  with a retry button — not silent failure.
- **US4:** As an operator, when I delete a task, I see a confirmation dialog first, then an
  undo toast after — no accidental deletions.
- **US5:** As an operator, when I click Assign or Delete, the button shows a loading state
  so I can't double-click and submit twice.
- **US6:** As an operator or end-user, I can install the console as a PWA and use it offline
  with my changes syncing when I reconnect.
- **US7:** As an end-user, when the network drops, I see a reconnect banner so I know my
  changes aren't being saved.
- **US8:** As an end-user, when I assign or delete a task, the board updates instantly
  (optimistically) even before the server responds.
- **US9:** As an end-user, I can see a sync status indicator in the navbar showing whether
  my data is synced, reconnecting, or disconnected.
- **US10:** As an end-user, when I make changes offline, they're queued and replay
  automatically when I reconnect — I can see how many are pending.
- **US11:** As a developer, the E2E test suite exercises the full auth flow, board
  management, and cross-node correctness so regressions are caught before they ship.
- **US12:** As a developer, the SC4 guard prevents any change from breaking the node
  frontend compilation.

## Success criteria

### Improvement 1: Error Resilience Layer

- **SC1:** App.tsx is wrapped in a root ErrorBoundary with styled fallback UI ("Something went
  wrong" + "Reload" button). Leaf-level boundaries wrap lazy-loaded pages. No uncaught render crash
  produces a white screen. → US1
- **SC2:** AuthGuard route guard wraps all NormalLayout children. Signed-out user navigating to
  `/nodes` or `/tasks` is redirected to `/login?return_to=<path>`. → US2
- **SC3:** All task assign/delete `catch {}` blocks are replaced with `toast.error()` from
  `sonner`, including retry action buttons. → US3
- **SC4:** Delete operations show a confirmation dialog (existing `alert-dialog` primitives)
  before executing. Post-delete shows an undo toast with 5s timeout (if undoable). → US4
- **SC5:** Assign/Delete buttons show `disabled` + `isPending` state to prevent double-click
  mutations. → US5

### Improvement 2: Offline-First PWA

- **SC6:** `vite-plugin-pwa` generates a valid service worker. Chrome DevTools > Application >
  Service Workers shows registered SW. `npm run build` produces `manifest.json` with app name
  "VK Swarm Console", theme color `#0f172a`, display `standalone`. → US6
- **SC7:** `useOnlineStatus()` hook exposes `{ isOnline, wasOffline, lastOnlineAt }`.
  NormalLayout renders a reconnect banner ("You're offline — changes will sync when reconnected")
  when `isOnline === false`. → US7
- **SC8:** Optimistic mutation helpers (`optimisticDelete`, `optimisticUpdate`) in
  `src/lib/electric/optimistic.ts` snapshot the collection before REST, apply locally via query
  client, roll back on error. Wired into Tasks.tsx assign/delete so the board updates instantly. → US8
- **SC9:** `useLiveQuery` is wrapped with `isStale` indicator. Navbar shows a sync status dot:
  green (synced), yellow (reconnecting), red (disconnected >30s). → US9
- **SC10:** Offline mutation queue (IndexedDB via `idb-keyval`): REST mutations that fail due
  to network error (not 4xx/5xx) are queued. On reconnect, queued mutations replay in order. Navbar
  shows queue badge ("N pending"). → US10
- **SC11:** `npm run build` produces a valid PWA. Lighthouse PWA score ≥ 90. → US6

### Improvement 3: Playwright E2E Test Suite

- **SC12:** `remote-frontend/e2e/auth.spec.ts` — full OAuth PKCE flow: (a) visit `/` →
  redirect to `/login`, (b) mock provider callback, (c) verify `/v1/oauth/web/init` POST contains
  `app_challenge`, (d) verify localStorage stores `access_token` after redeem, (e) verify redirect
  to `/nodes` after login, (f) logout → token cleared → redirect to `/login`. → US11
- **SC13:** `remote-frontend/e2e/board.spec.ts` — kanban board: (a) 4 columns visible, (b)
  tasks from mocked data in columns, (c) card click opens TaskDetail, (d) assign fires PATCH,
  (e) delete → confirmation → DELETE → card removed, (f) DELETE 500 → error toast, card stays. → US11
- **SC14:** `remote-frontend/e2e/cross-node.spec.ts` — cross-node correctness: (a) tasks from
  two different nodes in same column, (b) TaskDetail shows correct `node_id`, (c) output logs from
  different nodes render with node name labels. → US11
- **SC15:** `remote-frontend/e2e/sc4-guard.spec.ts` — SC4 regression guard: (a) `cd ../frontend
  && npx tsc --noEmit` exit 0, (b) `cd ../frontend && npm run lint` exit 0, (c) `cd ../frontend &&
  npx vitest run` exit 0. Executed in Playwright `globalSetup`. → US12
- **SC16:** `npx playwright test` all green (7+ test cases across 4 spec files). No flaky tests
  (3x CI-mode runs all pass). → US11

## Constraints

- **No new backend changes** where avoidable. All routes needed already exist (`/v1/profile`,
  `/v1/oauth/web/*`, `/v1/nodes`, `/v1/tasks/*`, `/v1/api/electric/v1/shape`). If a backend change
  is required (e.g., a health endpoint for sync status, CORS adjustments for SW), make it — "do
  everything that is needed, do not defer or skip."
- **SC4 guard is non-negotiable:** the node frontend (`frontend/`) must continue to compile and
  pass lint+tests. No modifications to `frontend/` beyond what's needed for the guard to pass.
- **No new dependencies beyond `sonner`** for Improvement 1. For Improvement 2: `vite-plugin-pwa`,
  `workbox-window`, `idb-keyval` are approved. For Improvement 3: `@playwright/test` is approved.
- **Bare JSON API contract** stays. Hive returns bare `Json(...)` (no envelope) — established by
  `vk-swarm-hive-ui`. New API clients follow same pattern.
- **Bearer token auth** stays: `localStorage['access_token']` + `Authorization: Bearer`.
- **`useLiveQuery`** (not `useCollection`) for Electric — `@tanstack/react-db` v0.1.92.
- **`src/app/` renamed to `src/ui/`** — per tournament finding SC7. Follow this convention.
- **Routes nest under `/v1`** — all API paths use `/v1` prefix.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **Search/filter, virtual scrolling, drag-and-drop, i18n init, missing meta tags, dual API
  layers** — these are recorded in the audit for future workstreams but are lower priority.
- **Node-frontend modifications** beyond SC4 guard — the node frontend is the HA fallback.
- **New backend services or database migrations** — this workstream is primarily frontend.
- **Redesigning the auth flow** — the existing OAuth PKCE flow stays.
- **Restyling or rebranding** — the existing visual design stays. This is hardening, not a restyle.

## Approach

Three independently implementable improvements, each with its own gate. Order is flexible — they
touch different subsystems — but Error Resilience first minimizes blast radius for the other two.

1. **Error Resilience Layer (Improvement 1):** wrap the app in an ErrorBoundary, add AuthGuard
   route protection, install `sonner` for toast feedback, replace silent catch blocks with error
   toasts, add confirmation dialogs for destructive actions, add button loading states.

2. **Offline-First PWA (Improvement 2):** configure `vite-plugin-pwa` for service worker +
   manifest, add `useOnlineStatus` hook + reconnect banner, build optimistic mutation helpers for
   Electric collections, add sync status indicator in Navbar, implement IndexedDB-backed offline
   mutation queue with `idb-keyval`.

3. **Playwright E2E Test Suite (Improvement 3):** install `@playwright/test`, write 4 spec files
   (auth, board, cross-node, SC4-guard), configure `playwright.config.ts` with mock API server.

Each improvement has its own independent gate — if one is completed before the others, it ships
without waiting. The full workstream is done when all three gates pass.

## Design / architecture

### Improvement 1: Error Resilience Layer

```
remote-frontend/src/
├── components/
│   ├── ErrorBoundary.tsx          # root + leaf boundaries, fallback UI
│   └── AuthGuard.tsx              # route guard, uses useProfile().isSignedIn
├── lib/
│   └── toast.ts                   # sonner wrapper (toast.error/success)
└── ui/                            # existing pages — wire toasts + dialogs here
    └── Tasks.tsx                  # replace catch{} with toast.error, add confirmation dialogs, loading states
```

**ErrorBoundary.tsx:** class component extending `React.Component<Props, State>`. On error, renders
a styled fallback card (Midnight Terminal palette) with "Something went wrong" message + "Reload"
button that calls `window.location.reload()`. Mounted in `main.tsx` wrapping `<App />`. Leaf
boundaries wrap lazy-loaded page components.

**AuthGuard.tsx:** functional component that calls `useProfile()` (from existing ProfileProvider,
task 101). If `!isSignedIn`, renders `<Navigate to="/login" state={{ returnTo: location.pathname }} />`.
Otherwise renders `children`. Wraps `{isSignedIn && <NormalLayout>}` in AppRouter — replaces the
current unprotected render at `AppRouter.tsx:160-168`.

**sonner integration:** install `sonner` (12KB zero-dependency toast library). Add `src/lib/toast.ts`
wrapper exporting `toast.error(message, { action: { label: 'Retry', onClick: fn } })` and
`toast.success(message, { action: { label: 'Undo', onClick: fn } })`. Mount `<Toaster />` in
`App.tsx` inside providers. Replace every silent `catch {}` block in Tasks.tsx with toast calls.

**Confirmation dialogs:** use existing `alert-dialog` primitives from `src/components/ui/` (shadcn
base, already in the dependency tree). Dialog appears before DELETE with "Are you sure?" prompt.
Post-delete undo toast with 5s timeout allows recovery.

**Loading states:** Add `isPending` boolean to mutation hooks. Buttons show `disabled={isPending}`
and spinner when pending. Prevents double-click mutations.

### Improvement 2: Offline-First PWA

```
remote-frontend/src/
├── lib/
│   ├── offline.ts                 # useOnlineStatus hook
│   └── electric/
│       ├── optimistic.ts          # optimisticDelete, optimisticUpdate
│       └── sync-status.ts         # useSyncStatus (wraps useLiveQuery + isStale)
├── components/
│   └── layout/
│       └── NormalLayout.tsx       # add reconnect banner
├── components/
│   └── layout/
│       ├── Navbar.tsx             # add sync status dot + queue badge
├── lib/
│   └── mutation-queue.ts          # IndexedDB queue + replay
├── vite.config.ts                 # vite-plugin-pwa config
├── manifest.json                  # generated by vite-plugin-pwa
└── public/
    └── icons/                     # PWA icons (192/512px)
```

**vite-plugin-pwa config:** stale-while-revalidate for SPA shell (`/`, `/login`, `/oauth/callback`),
cache-first for static assets (`/assets/*`), network-first for API routes (`/v1/*`). `manifest.json`
generated with name "VK Swarm Console", short_name "VK Swarm", theme_color `#0f172a`,
background_color `#0f172a`, display `standalone`, icons array with 192px + 512px maskable.

**useOnlineStatus hook:** watches `navigator.onLine` + `online`/`offline` events. Exposes
`{ isOnline: boolean, wasOffline: boolean, lastOnlineAt: Date | null }`. Used by NormalLayout
to conditionally render the reconnect banner.

**Reconnect banner:** slim bar at top of NormalLayout with muted yellow/orange styling ("You're
offline — changes will sync when reconnected"). Appears/disappears with `isOnline` transitions.
Dismissable but auto-reappears on next offline event.

**Optimistic mutations:** `optimisticDelete(collection, id)` snapshots current collection data,
applies delete locally via React Query's `queryClient.setQueryData`, calls REST DELETE, rolls
back on error. `optimisticUpdate(collection, id, patch)` does same for PATCH. Wired into
Tasks.tsx assign/delete handlers.

**Sync status indicator:** wraps `useLiveQuery` — if the Electric shape stream disconnects for
>30s, marks data as `isStale`. Navbar shows a colored dot: green (synced, <30s since last
update), yellow (reconnecting, 30s–60s), red (disconnected >60s).

**Offline mutation queue:** uses `idb-keyval` (1.5KB IndexedDB wrapper). Failed REST mutations
where error is a network error (not 4xx/5xx) are serialized and queued with operation type,
endpoint, payload, and timestamp. On reconnect (detected by `useOnlineStatus`), queued mutations
replay in FIFO order. Successful replay removes from queue. Failed replay (4xx/5xx from server)
removes from queue and shows toast error. Navbar shows badge "N pending" when queue > 0.

### Improvement 3: Playwright E2E Test Suite

```
remote-frontend/
├── e2e/
│   ├── auth.spec.ts               # OAuth PKCE flow (6 test cases)
│   ├── board.spec.ts              # kanban board (6 test cases)
│   ├── cross-node.spec.ts         # cross-node correctness (3 test cases)
│   └── sc4-guard.spec.ts          # SC4 regression guard in globalSetup
├── playwright.config.ts           # webServer, baseURL, projects
└── e2e/
    └── fixtures/
        ├── mock-api.ts            # MSW route handlers for /v1/*
        └── mock-electric.ts       # mock Electric shape data
```

**playwright.config.ts:** single chromium project. `webServer` starts Vite dev server.
`baseURL` = `http://localhost:5173`. `globalSetup` runs SC4 guard script. Timeout 30s per test.

**auth.spec.ts:** uses `page.route()` to intercept `/v1/oauth/web/init` and `/v1/oauth/web/redeem`.
Verifies full PKCE flow: init POST body contains `app_challenge`, redeem response stores token in
localStorage, post-login redirect to `/nodes`, logout clears token and redirects to `/login`.

**board.spec.ts:** routes `/v1/tasks/bulk` and `/v1/tasks/by-source` to return mock task data with
varying statuses and node_ids. Routes `/v1/electric/v1/shape` to return mock Electric sync data.
Tests: 4 columns visible with correct headers, task cards contain expected data, card click opens
TaskDetail with correct panels, assign button fires PATCH with correct body, delete shows
confirmation dialog then fires DELETE, DELETE 500 shows error toast and card remains.

**cross-node.spec.ts:** mock data includes tasks from 2 different nodes in same column.
Verifies: board renders both node's tasks, TaskDetail shows `node_id` label per task, output
logs from different nodes display with distinguishing labels.

**sc4-guard.spec.ts:** `globalSetup` (not a test) runs shell commands: `cd ../frontend && npx tsc
--noEmit` → assert exit 0, `cd ../frontend && npm run lint` → assert exit 0, `cd ../frontend &&
npx vitest run` → assert exit 0. If any fails, Playwright suite aborts before running tests.

### Gate summary

| Improvement | Gate command | Location |
|---|---|---|
| 1 + 2 | `cd remote-frontend && npx tsc --noEmit && npm run lint && npx vitest run` | remote-frontend/ |
| 3 | `cd remote-frontend && npx playwright test` (3x CI mode) | remote-frontend/ |
| SC4 | `cd frontend && npx tsc --noEmit && npm run lint && npx vitest run` | frontend/ |

All gates must be green before the workstream ships. Each improvement can be PR'd individually or
together — no ordering dependency between them.

## Decisions

- **D1: Error Resilience ships first.** It's the cheapest to fix and protects against user-visible
  breakage. The other two improvements depend on a working app. Reversible → no ADR.
- **D2: PWA uses Workbox strategies via vite-plugin-pwa, not a custom service worker.** The
  plugin generates a production-grade SW with minimal config. Reversible → no ADR.
- **D3: Offline mutation queue uses idb-keyval for persistence (not localStorage).**
  IndexedDB handles structured data, larger payloads, and transactions.
  Reversible → no ADR.
- **D4: E2E tests use MSW (Mock Service Worker) for API mocking, not a live hive instance.**
  Faster, deterministic, no infrastructure dependency. The next-session doc suggests an MSW
  or pytest fixture; MSW is chosen for JS-native test reliability. Reversible → no ADR.
- **D5: SC4 guard runs in Playwright globalSetup, not as a separate CI step.**
  The test suite won't even start if the node frontend is broken — catch regression at the
  earliest possible point. Reversible → no ADR.
- **D6: sonner for toasts (not shadcn toast).** sonner is smaller (12KB), has built-in action
  buttons and promise support, and avoids shadcn toast's `useToast` hook boilerplate.
  Reversible → no ADR.

No irreversible decisions — all changes are additive frontend code in `remote-frontend/`. No
deletes, no migrations, no wire-format changes, no contract changes. No ADRs required.

## Test strategy

### Unit tests (Vitest, existing framework)

- **ErrorBoundary:** mock a throwing child, assert fallback UI renders, assert "Reload" button
  exists and is clickable.
- **AuthGuard:** render with `isSignedIn = false`, assert redirect to `/login` with `returnTo`.
  Render with `isSignedIn = true`, assert children render.
- **useOnlineStatus:** mock `navigator.onLine` and dispatch `online`/`offline` events, assert
  hook returns correct `isOnline`, `wasOffline`, `lastOnlineAt`.
- **optimisticDelete/optimisticUpdate:** mock `queryClient.setQueryData`, call helper, assert
  data was optimistically mutated, mock REST error, assert rollback restores original data.
- **mutation-queue:** queue a mutation to idb-keyval, assert it's stored, trigger replay,
  assert it's dequeued after success, assert error toast on 4xx/5xx replay.
- **Toast wrapper:** assert `toast.error()` calls `sonner`'s `toast` with correct args.

### Integration tests (Vitest + React Testing Library)

- **Tasks.tsx with toasts:** render Tasks component, trigger assign mutation error, assert toast
  appears with retry button.
- **Tasks.tsx confirmation dialog:** trigger delete, assert dialog renders, confirm → assert
  DELETE dispatched, cancel → assert no dispatch.
- **Tasks.tsx loading states:** assert buttons show `disabled` attribute during mutation.
- **Navbar sync status:** mock `useSyncStatus` returning each state, assert dot color matches.
- **NormalLayout reconnect banner:** mock `useOnlineStatus` returning offline, assert banner
  renders.

### E2E tests (Playwright)

Full coverage described in SC12 through SC16 above. 15+ test cases across 4 spec files.
Configured with `retries: 2` in CI mode, `workers: 1` for deterministic execution. MSW runs
in the browser via Playwright's `page.route()` for API interception.
