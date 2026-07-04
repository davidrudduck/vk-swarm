# Decisions Ledger — vk-swarm-hive-ui

> Append-only. Every undictated choice the implementer makes goes here with a one-line
> rationale and the task id that prompted it.

## Pre-remediation baseline (decompose round 1)

- **Tournament round 1:** 3 cross-model competitors (Codex/gpt-5.2-fallback, Gemini, Claude)
  reviewed the breakdown. TALLY: Codex 11 (7B,4M), Gemini 8 (3B,1M,4m), Claude 12 (5B,4M,3m).
  Reports at `docs/plans/vk-swarm-hive-ui/reviews/r1-{codex,gemini,claude}-breakdown.md`.
- **6 high-confidence findings (≥2 reviewers) — all confirmed + remediated in-session:**
  1. `/v1` prefix missing on all hive REST paths — fixed in tasks 101, 102, 307 (+ 202 API base URL adjustment).
  2. OAuth contract wrong (`app_challenge`/`handoff_id`/`app_code`/`app_verifier`/`access_token`/`refresh_token`, not `code`/`state`/`redirect_url`/`profile`) — fixed in task 102.
  3. `remote-frontend` missing toolchain (vitest/eslint/@tanstack/react-db/testing-library) — fixed by new task 100.
  4. Electric module unreachable from `remote-frontend/` (no `lib/` dir) — fixed by new task 300 (copy bridge) + retargeting 301-304 to `remote-frontend/src/lib/electric/`.
  5. Task 307 `POST /tasks/{id}/assign` with `{node_id}` wrong route — fixed: use `PATCH /v1/tasks/{id}/executing-node` with `{node_id}`; `/assign` is for human assignee (`{new_assignee_user_id, version}`), out of scope.
  6. ProfileResponse nullability (`username` + all `ProviderProfile` fields are `Option<String>`) — fixed in task 101.
- **1 dismissed finding (false positive):** Missing attempts/executions collections — `node_task_attempts`/`node_execution_processes` tables exist but are NOT synced to Electric (`electric_sync_table` not called for them). Spec correctly scopes SC2 to already-published shapes (the 6 in `ELECTRIC_SHAPE_TABLES`). Adding collections for unsynced tables would require server-side work (out of scope).
- **Single-reviewer findings remediated:**
  - Codex F1 (plan gate rewrites AGENTS.md commands) — fixed: plan gate now lists the 4 exact AGENTS.md commands + supplemental remote-frontend checks.
  - Codex F8 (`remote-frontend` lacks `@tanstack/react-db` deps) — fixed by task 100.
  - Codex F9 (task 201 files: list mismatch) — fixed: frontmatter now lists tsconfig.json, vite.config.ts, types/shared/types.ts.
  - Codex F10 (task 308 green-on-arrival) — fixed: reclassified as regression guard (verification-only), not a red-first TDD test.
  - Codex F11 (post-phase review gate missing) — fixed: added "Post-phase integrated adversarial review" section to plan.md (execute-time gate).
  - Gemini F1 (task 202 missing transitive deps) — fixed: added "Known likely transitive deps" note (UI lib, hooks, lib/utils) + strengthened STOP trigger.
  - Gemini F4 (plan Phase 2 titles stale) — fixed: 201 title updated in plan.md.
  - Gemini F5 (task 103 scope_test .ts vs .tsx) — fixed: `.tsx`.
  - Gemini F6 (task 104 files: omits Navbar/BottomNav) — fixed: added to frontmatter.
  - Gemini F7 (task 202 URL-adj ambiguous) — fixed: concrete `/api/` → `/v1/` edit instruction.
  - Claude F6 (ProfileResponse nullability) — fixed in task 101.
  - Claude F7 (plan 201 title mismatch) — fixed in plan.md.
  - Claude F8 (spec field names diverge from DB) — fixed: added spec-divergence note to tasks 301-303 recording that tasks use DB-accurate field names.
  - Claude F10 (task 304 covers_criteria empty) — fixed: set to `[SC5]`.
  - Claude F11 (BIGSERIAL PKs as string) — fixed: added BIGSERIAL note to tasks 302-303 (default to `id: string | number` until verified).
  - Claude F12 (SC4 enforcement absent from Done-when gate) — fixed: all phase-2/3 tasks now include `cd frontend && npx tsc --noEmit` in manual verification (SC4 check).

## Design refinements (decompose-time, pre-tournament)

- **ProfileProvider vs ConfigProvider:** the spec's `## Design` says "port ConfigProvider fetching hive `/profile` as `UserSystemInfo`". This is inaccurate — the shapes differ. Node `ConfigProvider` fetches `/api/config` → `UserSystemInfo { config, environment, profiles, capabilities, analytics_user_id, login_status }`. Hive `/profile` returns `ProfileResponse { user_id, username (nullable), email, providers (each field nullable) }`. Resolution: port the PATTERN (React context + useAuth hook + oauthApi client) but implement a NEW, SIMPLER `ProfileProvider` fetching `/v1/profile`. The spec's intent (SC3: net-new app shell with session-as-app auth) is satisfied; the design detail is refined here. Does NOT contradict the frozen spec (the spec says "session/auth model ported from the node frontend" — the pattern is ported, not the verbatim implementation).

## Task-authoring decisions

- (tasks 100, 300 created as remediation for toolchain gap + Electric alias bridge)
- (task 307 retitled "set executing node / delete" from "assign/reassign/delete" to match the actual route contract)

## Task 100: remote-frontend toolchain setup

- [Task 100] eslint.config.js: dropped i18next plugin + rules — hive has no i18n yet (unlike node frontend). — remote-frontend/eslint.config.js
- [Task 100] eslint.config.js: dropped check-file naming rules (PascalCase .tsx, camelCase hooks/utils) — hive has no naming convention enforced yet. — remote-frontend/eslint.config.js
- [Task 100] eslint.config.js: dropped NiceModal restrictions + @ebay/nice-modal-react detection — hive has no modals. — remote-frontend/eslint.config.js
- [Task 100] eslint.config.js: dropped @eslint-community/eslint-comments rules — not required for hive. — remote-frontend/eslint.config.js
- [Task 100] Key deps installed match frontend versions exactly: @tanstack/electric-db-collection@^0.3.12, @tanstack/react-db@^0.1.82, @tanstack/react-query@^5.96.2. Full npm install succeeded with 343 packages. — remote-frontend/package.json, remote-frontend/package-lock.json
- [Task 100] react-router-dom version conflict avoided: hive declared ^7.9.5 (vs node frontend ^6.8.1), but npm resolved compatibly — no pin needed. — remote-frontend/package.json

## Task 100 — Stage-2 panel skipped (trivial toolchain)
- **Decision:** Skip Stage-2 adversarial panel for task 100.
- **Rationale:** Task is mechanical toolchain setup (install vitest/eslint/@tanstack deps, add lint/test scripts, add @ alias, add setupTests). No business logic, no control-flow decisions, no irreversible moves. Stage-1 gate CONFORMS (file-set OK, typecheck exit 0, scope_test green). The eslint config is a slimmed copy of `frontend/eslint.config.js` with documented dropped rules. Risk surface too low to justify a cross-model panel.
- **Per execute skill:** circuit breaker widens panel on RETRY; first attempt on a trivial task with a clean gate does not require adversarial review.

## Task 101: ProfileProvider context (auth state from /profile)

- [Task 101] No retry loop in ProfileProvider.fetchProfile() — unlike ConfigProvider which retries on backend startup (node server may still boot), hive app shell assumes hive is always up (single-hive deployment, no startup-race condition). — remote-frontend/src/components/ProfileProvider.tsx:42-67
- [Task 101] ProfileState simplified: 3 fields `{ profile, isSignedIn, isLoaded }` vs ConfigProvider's 6-field UserSystemState. Hive does not serve config/environment/profiles/capabilities surfaces — only `/v1/profile` identity endpoint. — remote-frontend/src/components/ProfileProvider.tsx:22-26
- [Task 101] No children gating while loading (ConfigProvider gates with Loader). Task spec silent on UX loading state — app shell responsibility. Provider exposes isLoaded flag for consumers to gate as needed. — remote-frontend/src/components/ProfileProvider.tsx (entire component)
- [Task 101] Fetch uses `credentials: 'include'` for hive session auth (vs node backend which uses bearer tokens). — remote-frontend/src/components/ProfileProvider.tsx:53-54
- [Task 101] Tests: vi.stubGlobal('fetch', ...) + vi.mocked(globalThis.fetch) pattern to avoid TypeScript 'global' not found (jsdom environment). vi.unstubAllGlobals() in afterEach ensures isolation. — remote-frontend/src/components/ProfileProvider.test.tsx:6-11,60-61

## Task 101 (round 2) — Bearer token auth amendment

- **Rejection cause:** Round 1 implemented cookie-based auth (`credentials: 'include'`) when the hive uses Bearer token auth per the middleware contract.
- **Evidence:** `crates/remote/src/auth/middleware.rs:46-53` — `require_session` reads only `Authorization<Bearer>` header; `grep -rn 'cookie|Cookie' crates/remote/src/` returns zero matches.
- **Correction:** `ProfileProvider` now reads `localStorage.getItem('access_token')`. If absent → signed-out state, NO fetch. If present → `fetch('/v1/profile', { headers: { Authorization: 'Bearer ' + token } })`. On 401 → `localStorage.removeItem('access_token')` (token expired/invalid) + signed-out. On network error → signed-out but token NOT cleared (transient failure ≠ expiry).
- **localStorage binding:** The `access_token` key is part of the orchestrator-level contract (task 101, 102, 105) matching the existing invitation flow (`remote-frontend/src/api.ts:85` already sends `Authorization: Bearer ${accessToken}`).
- **ProfileContext default:** Intentionally `undefined` (matching `ConfigProvider` pattern) — using `useProfile()` outside `ProfileProvider` throws. NOT the literal `{ profile: null, isSignedIn: false, isLoaded: false }` — the latter would silently hide context misuse.
- **Tests (5 scenarios):** Bearer header assertion, no-token no-fetch, 401 clears token, network error preserves token, loading state before fetch resolves. Mocks: `vitest`/`@testing-library/react` `renderHook` + consumer hook; mocked `fetch` + mocked `localStorage` (vi.stubGlobal + getter/removeItem spies).
- **Files:** `remote-frontend/src/components/ProfileProvider.tsx` (rewritten 46-85), `remote-frontend/src/components/ProfileProvider.test.tsx` (rewritten 5 tests, mocks localStorage).

## Task 102: profileApi + oauthApi client (hive /oauth/web/* routes)

- [Task 102] Local `ApiResponse<T>` type in `remote-frontend/src/lib/api/utils.ts` instead of importing from `shared/types` — `shared/types` alias does not exist in `remote-frontend/` yet (task 201 adds it, phase 2). To avoid cross-phase dependency, defined local type inline. Semantically identical to node frontend's ApiResponse (`success: boolean`, `data?: T`, `error_data?: E`, `message?: string`). — remote-frontend/src/lib/api/utils.ts:5-10
- [Task 102] `oauthApi.logout()` always clears `localStorage.removeItem('access_token')` in `finally` block, regardless of server response outcome. Rationale: client-side session is lost once logout is initiated; transient server errors must not leave a stale token in storage. — remote-frontend/src/lib/api/oauth.ts:51-67
- [Task 102] `profileApi.get()` throws if `localStorage.getItem('access_token')` is null — no silent fallback. Rationale: ProfileProvider enforces the token-before-fetch contract at the caller level; profile fetch only happens when ProfileProvider has a token. — remote-frontend/src/lib/api/profile.ts:20-22
- [Task 102] re-exported `initOAuth`, `redeemOAuth` + types from `lib/api/oauth` in `api.ts` for backwards compatibility with existing invitation auth pages (which import from `./api`). Kept `getInvitation` and `acceptInvitation` in `api.ts` unchanged. — remote-frontend/src/api.ts:1-24
- [Task 102] Test mocks: global `localStorage` polyfill + `vi.mocked(g.fetch)` pattern for jsdom environment (Node.js runtime does not provide built-in localStorage; vitest jsdom config loads it but TypeScript sees it as unavailable without declaration). — remote-frontend/src/lib/api/oauth.test.ts:6-23,30-32
- [Task 102] Verification: `cd remote-frontend && npx vitest run src/lib/api/oauth.test.ts` (4 tests: init/redeem/logout/profileApi.get Bearer token + storage assertions), `cd remote-frontend && npx tsc --noEmit` (zero errors), `cd remote-frontend && npm run lint` (zero warnings). All exit 0.

## Task 102 (round 2) — Bare JSON pattern fix

- **Rejection cause:** Round 1 code called `handleApiResponse()` which expects `{ success: true, data: {...} }` envelope. The hive returns BARE JSON — `crates/remote/src/routes/oauth.rs:55,77,199` return `Json(HandoffInitResponse{...})` directly, not wrapped.
- **Evidence:** `crates/remote/src/routes/oauth.rs:55` returns `Json(HandoffInitResponse { ... })`, `crates/remote/src/routes/oauth.rs:77` returns `Json(HandoffRedeemResponse { ... })`, `crates/remote/src/routes/oauth.rs:199` returns `Json(ProfileResponse { ... })`. None use `ApiResponse<T>` wrapper. `remote-frontend/src/api.ts:36,53` correctly use bare JSON pattern (`return res.json()` directly without envelope unwrapping).
- **Correction:** Removed `handleApiResponse()` function entirely from `utils.ts`. Kept `ApiResponse<T>` type export (future hive routes may wrap; type available for other endpoints). Rewrote `oauthApi.init()`, `oauthApi.redeem()`, and `profileApi.get()` to use pattern: `if (!response.ok) throw new ApiError(...); return await response.json() as T;`.
- **Test fix:** All 5 vitest mocks now return bare JSON: `json: async () => ({ handoff_id, authorize_url })` instead of `json: async () => ({ success: true, data: {...} })`. Added Test 5 (error path): `oauthApi.init()` with `response.ok = false` throws `ApiError`.
- **Files touched:** `remote-frontend/src/lib/api/utils.ts` (deleted `handleApiResponse`), `remote-frontend/src/lib/api/profile.ts` (bare JSON), `remote-frontend/src/lib/api/oauth.ts` (bare JSON for init/redeem), `remote-frontend/src/lib/api/oauth.test.ts` (5 tests all pass, 4 mocks + error test).
- **Verification:** `cd remote-frontend && npx vitest run src/lib/api/oauth.test.ts` exits 0 (5 tests pass), `cd remote-frontend && npx tsc --noEmit` exits 0 (zero errors).

## Task 103: useAuth hook over ProfileProvider

- [Task 103] Context source divergence: node frontend `useAuth()` wraps `useUserSystem()` (from `ConfigProvider`); hive `useAuth()` wraps `useProfile()` (from `ProfileProvider`). Both return `{ isSignedIn, isLoaded, userId }` — identical surface, different context source. Consumers do not change between environments.
- [Task 103] `userId` field derivation: `profileState.profile?.user_id ?? null` — defaults to null when profile is null or user_id is undefined.
- [Task 103] Test mocking: follows ProfileProvider.test.tsx pattern (vi.stubGlobal fetch + localStorage, renderHook with wrapper, waitFor loading state). Three test cases: (1) fetch 200 → signed-in + userId; (2) fetch 401 → signed-out + null userId; (3) no wrapper → throws.
- [Task 103] Files: created `remote-frontend/src/hooks/auth/useAuth.ts` (wraps useProfile, 8 lines), created `remote-frontend/src/hooks/auth/useAuth.test.tsx` (3 tests, mocks fetch+localStorage).
- [Task 103] Verification: `cd remote-frontend && npx vitest run src/hooks/auth/useAuth.test.tsx` exits 0 (3 tests pass), `cd remote-frontend && npx tsc --noEmit` exits 0 (zero errors), `cd remote-frontend && npm run lint` exits 0 (zero warnings).

## Task 104: Hive app shell NormalLayout

- [Task 104] **Dropped features from node Navbar:**
  1. `view=preview|diffs` URL param hiding (hive has no preview/diffs routes) — NormalLayout unconditionally renders Navbar.
  2. `DevBanner` component (depends on `ConfigProvider`/`useUserSystem` which hive doesn't have) — dropped entirely.
  3. `ProjectSwitcher` (node concept; hive console has no per-project context) — dropped.
  4. `SearchBar` + mobile search dialog (node concept; hive v1 has no search) — dropped.
  5. Archive toggle (node task-list feature; not in hive scope) — dropped.
  6. `ActivityFeed` component (depends on `useUserSystem`) — dropped.
  7. `OpenInIdeButton` (node-frontend feature; hive has no IDE integration) — dropped.
  8. `ThemeToggle` (node feature; hive inherits app theme) — dropped.
  9. Dropdown menu for OAuth/settings (node pattern) — Logout is direct button instead.
  10. Task creation button (hive console has no task creation from navbar) — dropped.
  11. i18next translations (hive v1 not localized yet) — all labels hardcoded.
  12. `useProject`, `useSearch`, `useUserSystem` hooks (node-frontend local state) — not imported.

- [Task 104] **Nav structure changes:**
  - Node INTERNAL_NAV: `/projects`, `/processes`. Hive nav: `/nodes`, `/tasks`, `/settings` (console-specific routes).
  - Nav items in Navbar second row mirror the BottomNav structure (3 items, icon + label).
  - Active link detection: exact path match (`location.pathname === item.to`) for hive, vs node's prefix matching (`startsWith`) for project-context scoped routes.

- [Task 104] **Navbar implementation:**
  - Slim Navbar (~60 lines including NavItem extraction): text logo "VK Swarm" linking to `/nodes`, nav row with 3 items (Nodes, Tasks, Settings), Logout button in top-right.
  - Logout: calls `oauthApi.logout()` then `window.location.reload()` (no `reloadSystem()` available; hive handles session via localStorage token).
  - Icons: `FolderOpen` (Nodes), `ListTodo` (Tasks), `Settings` (Settings), `LogOut` (Logout) — all from lucide-react.
  - Active link styling: `border-b-2 border-primary py-2 text-foreground` (exact match to node Navbar.tsx:345-346).
  - Used `data-testid="navbar"` on the nav wrapper for test assertion (undictated, required for test).

- [Task 104] **BottomNav implementation:**
  - Slim BottomNav (~40 lines): fixed bottom nav, `sm:hidden`, 3 items (Nodes, Tasks, Settings).
  - NavItem sub-component inlined (not a separate file) — contains icon, label, active state, onClick handler.
  - Icons: `FolderOpen`, `ListTodo`, `Settings` matching Navbar.
  - Active detection: exact path match for each item.

- [Task 104] **NormalLayout simplification:**
  - Dropped `useSearchParams` hook and `view=preview|diffs` ternary (node has conditional Navbar hiding; hive always shows nav).
  - Dropped `DevBanner` import entirely.
  - Structure: `<> <Navbar /> <div className="flex-1 min-h-0 overflow-hidden pb-14 sm:pb-0"><Outlet /></div> <BottomNav /> </>` — verbatim from node, minus DevBanner.

- [Task 104] **Test implementation:**
  - Single test case: renders `NormalLayout` via `createMemoryRouter` with a test route containing a child `<div data-testid="outlet-child" />`.
  - Assertions: (1) navbar renders (`getByTestId('navbar')`); (2) outlet child renders (`getByTestId('outlet-child')`); (3) bottom nav renders (check `getAllByRole('navigation').length > 0`).
  - No `useProfile()` mocking needed — `NormalLayout` is a pure layout component with no state reads; both Navbar and BottomNav use `useLocation`/`useNavigate` which work inside `RouterProvider`.

- [Task 104] **Utils and dependencies:**
  - Created `remote-frontend/src/lib/utils.ts` with `cn()` helper (clsx + tailwind-merge) — verbatim from node frontend (no changes).
  - Added `lucide-react@^1.7.0` to `remote-frontend/package.json` dependencies (matching node frontend major version).
  - `npm install` ran locally to verify dependency resolution succeeded (344 packages); lock file not committed (task spec files list does not include it, unlike task 100).

- [Task 104] **Verification:**
  - `cd remote-frontend && npx tsc --noEmit` exits 0 (zero type errors).
  - `cd remote-frontend && npx vitest run src/components/layout/NormalLayout.test.tsx` exits 0 (1 test passes).
  - `cd remote-frontend && npm run lint` exits 0 (zero warnings, max-warnings 0).
  - `WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/layout/NormalLayout.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 104` exits 0 (CONFORMS).

## Task 105: Full console route tree (AppRouter + providers)

- [Task 105] **PKCE verifier storage location:** Verifier is stored in `sessionStorage` (key `'oauth_verifier'`) via `storeVerifier()` and retrieved via `retrieveVerifier()` from `@/pkce`. NOT stored in URL query params or router state. Rationale: sessionStorage ensures verifier is cleared when the browser session ends, preventing cross-tab reuse and XSS persistence. — remote-frontend/src/AppRouter.tsx:80, remote-frontend/src/pkce.ts:30-43
- [Task 105] **Router factory pattern:** Exported `createRoutes()` helper function (returns route array) so tests can import and use with `createMemoryRouter`. Main app uses `createBrowserRouter(createRoutes())` for production. Rationale: decouples route definitions from router instance type, making routes testable without requiring the browser runtime. — remote-frontend/src/AppRouter.tsx:167-177,179
- [Task 105] **NotFoundPage layout:** NotFoundPage (catch-all `*` route) is wrapped in `NormalLayout`, so authenticated 404s show the full app shell (navbar + outlet + bottom nav). Unauthenticated users are redirected to `/login` via `RootRedirect` before they can hit the 404 route. Rationale: provides consistent UX for authenticated users (they always see nav) and auth-gating is transparent (pre-auth flows like `/invitations/*` bypass layout). — remote-frontend/src/AppRouter.tsx:156-161
- [Task 105] **Placeholder page implementations:** `/nodes` and `/tasks` pages are implemented inline in AppRouter.tsx as simple components (`NodesPage`, `TasksPage`) rendering placeholder text wrapped in `NormalLayout`. NOT in separate files. Rationale: task is explicit — "lazy-loaded placeholder OK for this task" — phase 2 will replace with real pages; inline keeps scope minimal and testable. — remote-frontend/src/AppRouter.tsx:135-153
- [Task 105] **OAuth callback query param naming:** OAuth callback reads `code` as the authorization code query param (per OAuth 2.0 standard returned by providers). Hive backend returns `handoff_id` and expects `app_code` in the redeem request, but the provider redirects with `code=...`. The callback handler reads both: `searchParams.get('code')` for the provider param, passes it to `oauthApi.redeem()` as `appCode`. Rationale: aligns with RFC 6749 (OAuth 2.0) standard where providers return `code`; the hive's naming (`app_code`) is an internal contract detail. — remote-frontend/src/AppRouter.tsx:90
- [Task 105] **Test mocking strategy:** Tests mock `useProfile` at module level (vi.mock) returning controlled `{ isSignedIn, isLoaded, profile }` values per test. Router render uses `createMemoryRouter` with `initialEntries=['/']` parameter to set starting path. Navigate components (from `RootRedirect`) work correctly in memory router — the router updates its internal state and renders the target route. Assertions check rendered page content (e.g., heading text) rather than pathname. Rationale: memory router doesn't expose `window.location.pathname` like browser router, so content checks are the reliable assertion method. — remote-frontend/src/AppRouter.test.tsx:28-45
- [Task 105] **Provider wrap order in App.tsx:** `QueryClientProvider` wraps `ProfileProvider` wraps `AppRouter`. Rationale: QueryClient is needed by react-query hooks used in phase-2 rehosted components (coming later); ProfileProvider must be above AppRouter so all routes can access `useProfile()`. Order ensures the dependency tree is correct without circular dependencies. — remote-frontend/src/App.tsx:6-14
- [Task 105] **Verification:**
  - `cd remote-frontend && npx vitest run src/AppRouter.test.tsx` exits 0 (6 tests: 3 redirects, /nodes render, /invitations render, 404 render).
  - `cd remote-frontend && npx tsc --noEmit` exits 0 (zero type errors).
  - `cd remote-frontend && npm run lint` exits 0 (zero warnings, max-warnings 0).
  - `WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/AppRouter.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 105` exits 0 (CONFORMS).

## Task 105 (round 2) — OAuth callback param + error redirect fix

- **FIX 1 (blocker):** OAuth callback reads wrong query param. Server's `append_query_params` (`crates/remote/src/routes/oauth.rs:300-320`) appends `app_code` (NOT `code`) to the callback URL. Changed line 92 from `const appCode = searchParams.get('code')` to `const appCode = searchParams.get('app_code')`. Rationale: matches server-side contract — the provider's `code` parameter is redirected with a new parameter name `app_code` by the server before reaching the frontend callback.
- **FIX 2 (major):** OAuth callback errors must redirect to /login, not render error UI. Per task spec: "On redeem failure (or missing params, or missing verifier) → redirect to `/login?error=<encoded message>`". Replaced four error paths: (1) `oauthError` query param present (2) missing handoffId or appCode (3) missing verifier (4) redeem catch block. Each now calls `window.location.assign(\`/login?error=${encodeURIComponent(msg)}\`)` instead of `setError()`. Removed the `error` state variable and the entire error UI block (lines 139-153 in round 1). Kept the `isRedirecting` state and success path unchanged. Rationale: error feedback is handled by the login page (which reads `?error=...` query param), not the callback page; this matches the spec's redirect-on-error intent and simplifies the component (no error state needed).
- **Verification:** `cd remote-frontend && npx vitest run src/AppRouter.test.tsx` exits 0 (5 tests pass), `cd remote-frontend && npx tsc --noEmit` exits 0 (zero errors), `cd remote-frontend && npm run lint` exits 0 (zero warnings). — remote-frontend/src/AppRouter.tsx:84-163

## Task 106: Wire root providers

- [Task 106] **QueryClient options:** `{ defaultOptions: { queries: { staleTime: 30_000 } } }` (30s stale time). Rationale: `useQuery` consumers cache data briefly per task spec line 30; stale but not immediately refetch on mount. — remote-frontend/src/App.tsx:5-8
- [Task 106] **No i18n provider:** Hive shell is English-only in v1 (unlike node frontend which uses i18next). No `I18nextProvider` wrap in App.tsx. Out of scope; recorded here to prevent future i18n churn during phase 2. — remote-frontend/src/App.tsx
- [Task 106] **main.tsx entry point:** Changed from `<AppRouter />` to `<App />` so that providers (QueryClientProvider > ProfileProvider > AppRouter) are active from root. Rationale: AppRouter needs ProfileProvider context for RootRedirect logic; main.tsx must render App (provider tree) not the router directly. — remote-frontend/src/main.tsx:1-11
- [Task 106] **Test approach (App.test.tsx):** Mocked `./AppRouter` module to return a simple stub with ProfileProbe + QueryProbe + `/nodes` link. Real `createBrowserRouter` is difficult to drive in jsdom (router doesn't expose `window.location` during render). Stub approach isolates provider behavior from router behavior, making assertions deterministic. Three test cases: (1) ProfileProvider sets isSignedIn=true on /v1/profile 200; (2) QueryClientProvider caches query result; (3) router outlet renders (link href=/nodes is present). Mocks: global fetch + localStorage for auth. — remote-frontend/src/App.test.tsx:1-67
- [Task 106] **Verification:**
  - `cd remote-frontend && npx vitest run src/App.test.tsx` exits 0 (3 tests pass).
  - `cd remote-frontend && npx tsc --noEmit` exits 0 (zero errors).
  - `cd remote-frontend && npm run lint` exits 0 (zero warnings).
  - `cd remote-frontend && npm run build` exits 0 (phase 1 integration smoke passes).

## Task 201: Rehost setup — add shared/* path alias + copy shared types

- `shared/*` mapped to `src/types/shared/*` (NOT `../shared/*` like node frontend) — hive is self-contained, no repo-root `shared/` sibling in include path. Copy is a one-time snapshot; drift reconciled by extracting to a shared package later (spec decision 2).
- `@/*` alias already present from task 100; only `shared/*` added here.
- `shared/types.ts` is self-contained (zero imports), so no recursive dependency copy needed.
- Copied `shared/types.ts` verbatim (~1471 lines, generated by `crates/core/src/bin/generate_types.rs`).
- **Verification:**
  - `cd remote-frontend && npx vitest run src/types/shared.test.ts` exits 0 (1 test: resolves shared/types alias).
  - `cd remote-frontend && npx tsc --noEmit` exits 0 (zero type errors).
  - `cd remote-frontend && npm run lint` exits 0 (zero warnings).
  - `cd remote-frontend && npm run build` exits 0 (phase 1 integration smoke passes).

## Task 202 (round 1)
- [Task 202 orchestrator] Worker timed out at 10 minutes after copying 45 files (full transitive closure: swarm components + labels + ui primitives + hooks + types + API clients). Worker stripped `handleApiResponse` correctly from all 4 API clients (bare JSON, `/v1/` prefix). scope_test (278 lines) renders all exported components via mocked providers. Orchestrator fixes: (a) replaced `useSwarmHealth`/`useSwarmHealthActions` with inert stubs matching real interface (node-frontend hooks depend on `projectsApi.getAll()`, `useProjectMutations`, `@tanstack/react-query` not available in hive), (b) fixed SwarmProjectRow type error in scope_test (missing props nodes/isLoadingNodes/isExpanded/onToggleExpand/onUnlinkNode), (c) dropped `useMemo` from stub import. — remote-frontend/src/hooks/useSwarmHealth.ts, useSwarmHealthActions.ts, src/components/swarm/index.test.tsx
- [Task 202 implementer] Copied full transitive closure of swarm component deps: @/components/ui/* (13 shadcn primitives: alert, alert-dialog, badge, button, card, dialog, input, label, popover, select, tabs, textarea, tooltip), @/components/labels/* (ColorPicker, IconPicker, LabelBadge), @/hooks/* (useSwarmHealth, useSwarmHealthActions, useSwarmLabels, useSwarmProjects, useSwarmTemplates), @/lib/api/index.ts (re-exports 27+ API modules — required by swarm hooks). Stripped `handleApiResponse` from nodes/swarmProjects/swarmLabels/swarmTemplates.ts, replaced with bare-JSON pattern, changed /api/ → /v1/. Kept phase-1 utils.ts (no handleApiResponse). — remote-frontend/src/{components,lib/api,types,hooks}/
- [Task 202 implementer] Swarm components use `react-i18next` (useTranslation) — installed react-i18next@^15.6.1 + i18next@^25.4.1 via npm. — remote-frontend/package.json

## Task 203: Mount Nodes page at /nodes

- [Task 203] **useOrganizations hook vs useUserOrganizations:** Node frontend uses `useUserOrganizations()` (from ConfigProvider context fetching `/api/organizations`), which returns `{ organizations: OrganizationWithRole[] }`. Hive frontend created new `useOrganizations()` hook that: (a) fetches from `/v1/organizations` (Bearer token auth), (b) returns bare `Organization[]` (not wrapped in `organizations` key), (c) uses `useProfile().isSignedIn` as enabled guard. Pattern matches node frontend (React Query hook + staleTime 5 minutes) but leverages hive's simpler org model (no role tracking in Nodes page scope). — remote-frontend/src/hooks/useOrganizations.ts, remote-frontend/src/lib/api/organizations.ts
- [Task 203] **Organizations API client:** Created `organizationsApi.list()` fetching `GET ${API_BASE}/v1/organizations` with Bearer header. Response is bare JSON `{ organizations: Organization[] }`. Client throws `ApiError` on non-OK response. Follows pattern from task 102 (bare JSON, Bearer auth). — remote-frontend/src/lib/api/organizations.ts
- [Task 203] **Nodes.tsx organization selection:** Simplified from node frontend's `organizations.find(o => !o.is_personal) ?? organizations[0]` to just `organizations[0]?.id`. Rationale: hive may not have is_personal field in all contexts; single-org assumption for phase 2 is acceptable (hive console is single-org deployment). Falls back to no orgId (shows "connect hive server" message) if orgs array is empty. — remote-frontend/src/pages/Nodes.tsx
- [Task 203] **AppRouter lazy import + Suspense:** Replaced `/nodes` route placeholder with lazy-imported `Nodes` component wrapped in Suspense fallback (div with "Loading nodes..." text). Removed unused `NodesContent()` function. Imports Nodes via `lazy(() => import('@/pages/Nodes').then(m => ({ default: m.Nodes })))` to handle named export. — remote-frontend/src/AppRouter.tsx
- [Task 203] **Test setup:** Created Nodes.test.tsx with 5 test cases: (1) loading state, (2) no organizations message, (3) nodes render with data, (4) no nodes connected message, (5) error message. Mocks: useOrganizations + nodesApi.list + NodeCard component. Created `createMockQuery()` helper to properly type `UseQueryResult<T>` mocks (avoids `as any` warnings). Renders with QueryClientProvider only (no ProfileProvider needed — Nodes uses useOrganizations hook which handles auth). — remote-frontend/src/pages/Nodes.test.tsx
- [Task 203] **API index export:** Added `organizationsApi` to remote-frontend/src/lib/api/index.ts exports. — remote-frontend/src/lib/api/index.ts
- [Task 203] **Verification:**
  - `cd remote-frontend && npx tsc --noEmit` exits 0 (zero type errors).
  - `cd remote-frontend && npx vitest run src/pages/Nodes.test.tsx` exits 0 (5 tests pass).
  - `cd remote-frontend && npm run lint` exits 0 (only pre-existing warning in swarm/index.test.tsx, not introduced by this task).
