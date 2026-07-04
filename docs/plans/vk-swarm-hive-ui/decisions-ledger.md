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
