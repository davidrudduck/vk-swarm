---
id: "016"
phase: 4
title: "Gate the frontend on public auth state and tear down protected traffic on 401"
status: ready
depends_on: ["008","012"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/lib/api/browserAuth.ts"
  - "frontend/src/components/auth/AuthBoundary.tsx"
  - "frontend/src/components/auth/__tests__/AuthBoundary.test.tsx"
  - "frontend/src/lib/api/utils.ts"
  - "frontend/src/lib/api/index.ts"
  - "frontend/src/App.tsx"
siblings: ["frontend/src/lib/api/oauth.ts","frontend/src/components/ConfigProvider.tsx","frontend/src/lib/api/approvals.ts","frontend/src/lib/api/attempts.ts","frontend/src/lib/api/backups.ts"]
irreversible: false
scope_test: "frontend/src/components/auth/__tests__/AuthBoundary.test.tsx"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS5"]
---
## Failing test (write first)
File: `frontend/src/components/auth/__tests__/AuthBoundary.test.tsx` — create executable Vitest/jsdom tests using `render`, `screen`, `fireEvent`, `waitFor`, and fake timers where polling time advances.

Required tests:
1. Unauthorized mount calls only `GET /api/auth/state`, renders `login-shell`, and never calls `/api/info`, `/api/auth/status`, projects, SSE, or WS.
2. Clicking `login-start` calls `browserAuthApi.startLogin('github', `${window.location.origin}/api/auth/handoff/complete`)` exactly once, opens the returned `authorize_url` in a popup, and polls only public `/api/auth/state`.
3. Poll responses false,false,true cause protected children to mount only after true; assert no protected request before then. After children mount, advance timers by 5000ms and assert `getState` call count is unchanged (pins "stop polling on authorization").
4. Popup close while still unauthorized stops polling and keeps the login shell.
5. Deadline expiry stops polling and keeps the login shell.
6. Unmount clears interval/deadline and closes no unrelated window; advancing timers makes no further calls.
6b. Render under `React.StrictMode`. After the initial `getState` resolves, the login-shell is visible (the live effect must still accept the response). The app entry (`frontend/src/main.tsx:19-26`) wraps `<App />` in StrictMode, which replays setup-cleanup-setup on the same instance.
6c. If `startLogin` is still awaiting when the component unmounts, the continuation must not call `window.open` and must not install interval/deadline timers.
6d. Two clicks of `login-start` (first startLogin already resolved so an interval is live) must leave exactly one poll interval. After the second click resolves, advance one `POLL_INTERVAL_MS` and assert `getState` increased by exactly 1 (two live intervals would double-fire).
7. OAuth unavailable hides `login-start`.
8. `notifyUnauthorized()` tears down already-mounted children back to login shell.
9. Existing `makeRequest` 401 notification fires exactly once.

Mock `window.open` with a controlled `{ closed }` object and spy on `browserAuthApi.startLogin/getState`; do not mount `UserSystemProvider` in these unit tests. The click/init/popup/poll/render sequence must be executable—no comment placeholders.


## Change
**File:** `frontend/src/lib/api/browserAuth.ts` — create the namespace-object API matching `oauth.ts`. Import `BrowserAuthState` from `shared/types`. Each method uses `makeRequest` + `handleApiResponse` (or the `oauthApi.logout` throw-`ApiError`-if-`!ok` pattern for the two POSTs that return void):
```ts
export const browserAuthApi = {
  getState: async (): Promise<BrowserAuthState> => {
    const response = await makeRequest('/api/auth/state');
    return handleApiResponse<BrowserAuthState>(response);
  },
  startLogin: async (provider: string, returnTo: string):
    Promise<{ handoff_id: string; authorize_url: string }> => {
      const response = await makeRequest('/api/auth/handoff/init', {
        method: 'POST', body: JSON.stringify({ provider, return_to: returnTo }),
      });
      return handleApiResponse(response);
    },
  logout: async (): Promise<void> => {
    const response = await makeRequest('/api/auth/browser/logout', { method: 'POST' });
    if (!response.ok) {
      throw new ApiError(`Logout failed with status ${response.status}`, response.status, response);
    }
  },
  disconnectHive: async (): Promise<void> => {
    const response = await makeRequest('/api/auth/logout', { method: 'POST' });
    if (!response.ok) {
      throw new ApiError(`Logout failed with status ${response.status}`, response.status, response);
    }
  },
};
```

**File:** `frontend/src/components/auth/AuthBoundary.tsx` — create a complete unauthorized login shell in this task. Initial mount calls only `browserAuthApi.getState()`. `login-start` calls:
```ts
const returnTo = `${window.location.origin}/api/auth/handoff/complete`;
const { authorize_url } = await browserAuthApi.startLogin('github', returnTo);
const popup = window.open(authorize_url, 'hive-oauth', 'popup,width=600,height=720');
```
Poll **only** public `getState()` on a bounded interval until `authorized === true`, popup closes, deadline expires, or component unmounts. Locked constants (do not invent others): `POLL_INTERVAL_MS = 1000`, `LOGIN_DEADLINE_MS = 10 * 60 * 1000` (spec handoff TTL). Login-shell markup uses `data-testid="login-shell"`; the start button uses `data-testid="login-start"`. On true, stop polling and mount protected `children`. Popup-close/deadline remain on the login shell and stop polling. Cleanup clears every timer and prevents state updates after unmount. The app runs under `React.StrictMode` (`frontend/src/main.tsx:19-26`). A `mountedRef` that is set `false` in cleanup and never restored is forbidden: StrictMode cleanup+replay is the same instance, so the live effect would ignore `getState` and abort every poll. Locked repair: set `mountedRef.current = true` at the start of the mount effect (before registering `onUnauthorized` or calling `getState`). After `await startLogin(...)`, if `!mountedRef.current`, return without `window.open` and without installing timers. `startLogin` must call the shared `stopPolling` before installing a new interval/deadline so a second click cannot orphan the first interval. Never call `/api/info` or `/api/auth/status` from the unauthorized shell. The existing protected `OAuthDialog` remains unchanged for already-authorized app workflows.

**File:** `frontend/src/lib/api/utils.ts` — add a single module-level unauthorized handler and two exports; change nothing else in this file:
```ts
let unauthorizedHandler: (() => void) | null = null;
export function onUnauthorized(handler: () => void): () => void {
  unauthorizedHandler = handler;
  return () => {
    if (unauthorizedHandler === handler) unauthorizedHandler = null;
  };
}
export function notifyUnauthorized(): void {
  unauthorizedHandler?.();
}
```
Inside `makeRequest`, after a successful `fetch` (before the `return`), if `response.status === 401` call `notifyUnauthorized()` exactly once for that response, then return the `Response` unchanged. Do not throw. Do not change timeout, headers, signal, or `handleApiResponse`.

**File:** `frontend/src/App.tsx`
**Anchor:** current `function App()` at **L254-274**, wrapper JSX **L256-272**. Wrap `UserSystemProvider` with `AuthBoundary` directly inside `BrowserRouter`, including matching closing tag. This location is load-bearing because `UserSystemProvider` performs protected bootstrap.

**File:** `frontend/src/lib/api/index.ts` — export `browserAuthApi`.

**Symbol grounding:** task introduces `browserAuthApi.getState/startLogin/logout/disconnectHive`, `AuthBoundary`, `onUnauthorized`, and `notifyUnauthorized`. Tests execute click/init/window.open/public-state polling and final children render.


## Allowed moves
[
  "Create browserAuth.ts, AuthBoundary.tsx and the test file.",
  "In utils.ts: add the 401 inspection inside makeRequest and the two handler-registry exports. Change nothing else in that file.",
  "In App.tsx: add exactly the AuthBoundary wrapper and its import.",
  "In index.ts: add exactly one export line."
]


## STOP triggers
[
  "AuthBoundary issuing any request other than GET /api/auth/state before it is authorized.",
  "Putting the boundary INSIDE UserSystemProvider — the provider's mount effect fires the protected /api/info call.",
  "Storing the auth state, any token, or any profile in localStorage/sessionStorage — the session lives in an HttpOnly cookie the JS must never see.",
  "Adding `credentials: 'include'` or any CORS/origin handling — same-origin fetch already sends the cookie, and cross-origin is explicitly out of scope.",
  "Changing any existing behaviour of makeRequest other than the added 401 notification.",
  "A test asserting the login shell by CSS class or visible text instead of a stable data-testid.",
  "Leaving login-start as visual-only or comment-stub behavior — task 016 must deliver the working public initiation/popup/poll flow.",
  "Polling /api/info or /api/auth/status; the unauthorized loop may call only public /api/auth/state.",
  "Failing to stop polling on authorization, popup close, deadline or unmount.",
  "Editing or replacing the existing protected OAuthDialog; it remains available after authorization.",
  "Using the stale App L137-156 anchor; current App() is L254-274 and wrapper JSX L256-272.",
  "Inventing poll/deadline constants other than POLL_INTERVAL_MS = 1000 and LOGIN_DEADLINE_MS = 10 * 60 * 1000.",
  "Throwing from makeRequest on 401, or notifying more than once per 401 response.",
  "A mountedRef that stays false after StrictMode cleanup+replay, so the live effect ignores getState.",
  "Installing a new login poll interval without first clearing any existing interval (orphaned poll after second click).",
  "Calling window.open or installing timers after startLogin resolves if the component has unmounted."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `cd frontend && npx vitest run src/components/auth/__tests__/AuthBoundary.test.tsx` — 12 executable tests green (original 9 plus 6b StrictMode, 6c unmount-during-startLogin, 6d single remaining poll after second click).
2. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="cd frontend && npx vitest run {scope}" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 016` exits 0. (The runner must be pinned explicitly: a `.test.tsx` scope would otherwise be dispatched to `node --test`, which cannot execute TSX.)
3. `cd frontend && npm run lint && npx tsc --noEmit && npx vitest run` — all green.
4. Manual: run the node with an unauthorized browser and confirm the network panel shows exactly one `/api/auth/state` request and no `/api/info`, no SSE, no WS.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 016` exits 0
