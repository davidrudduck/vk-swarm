---
id: "102"
phase: 1
title: "Hive app shell: profileApi + oauthApi client (hive /oauth/web/* routes)"
status: done
depends_on: ["100"]
parallel: false
conflicts_with: []
files:
  - frontend/src/lib/api/oauth.ts
  - frontend/src/lib/api/config.ts
  - frontend/src/lib/api/utils.ts
  - remote-frontend/src/lib/api/profile.ts
  - remote-frontend/src/lib/api/oauth.ts
  - remote-frontend/src/lib/api/utils.ts
  - remote-frontend/src/lib/api/oauth.test.ts
  - remote-frontend/src/api.ts
irreversible: false
scope_test: "remote-frontend/src/lib/api/oauth.test.ts"
allowed_change: edit
covers_criteria: [SC3]
---
## Failing test (write first)
File: `remote-frontend/src/lib/api/oauth.test.ts`

Tests mock global `fetch` and assert:
1. `oauthApi.init(provider, returnTo, appChallenge)` POSTs `/v1/oauth/web/init` with `{ provider, return_to: returnTo, app_challenge: appChallenge }` and returns `HandoffInitResponse` shape `{ handoff_id: string, authorize_url: string }` (BARE JSON — mock `response.json()` returns `{ handoff_id, authorize_url }` directly, NOT `{ success: true, data: {...} }`; the hive returns `Json(HandoffInitResponse{...})` per `crates/remote/src/routes/oauth.rs:55`).
2. `oauthApi.redeem(handoffId, appCode, appVerifier)` POSTs `/v1/oauth/web/redeem` with `{ handoff_id: handoffId, app_code: appCode, app_verifier: appVerifier }` and returns `HandoffRedeemResponse` shape `{ access_token: string, refresh_token: string }` (BARE JSON — mock returns the object directly, no envelope).
3. `oauthApi.logout()` POSTs `/v1/oauth/logout` with `Authorization: Bearer <localStorage.getItem('access_token')>` and then clears `localStorage.removeItem('access_token')`.
4. `profileApi.get()` GETs `/v1/profile` with `Authorization: Bearer <localStorage.getItem('access_token')>` (NOT `credentials: include` — the hive uses Bearer token auth, see task 101 "Token storage contract") and returns `ProfileResponse` (BARE JSON — mock returns the object directly; with nullable `username` + nullable `ProviderProfile` fields per `crates/utils/src/api/oauth.rs:49-63`).
5. Error path: `oauthApi.init` with `response.ok = false` (e.g. 400) throws an `ApiError` (do NOT swallow — caller needs to surface login failure).

**Contract source:** `crates/utils/src/api/oauth.rs:7-33` defines `HandoffInitRequest{provider, return_to, app_challenge}`, `HandoffInitResponse{handoff_id, authorize_url}`, `HandoffRedeemRequest{handoff_id, app_code, app_verifier}`, `HandoffRedeemResponse{access_token, refresh_token}`. The existing `remote-frontend/src/api.ts:13-21,37-55` already implements this correctly — read it as the reference sibling. The `/v1` prefix is required (`crates/remote/src/routes/mod.rs:112-113`).

## Change
- **File:** `remote-frontend/src/lib/api/utils.ts` (CREATE)
  - **Before:** (file does not exist)
  - **After:** Port `ApiError`, `makeRequest` from `frontend/src/lib/api/utils.ts`. DO NOT port `handleApiResponse` — it expects a `{success, data}` envelope that the hive does NOT use. The hive returns bare JSON (`crates/remote/src/routes/oauth.rs:55,77,199` return `Json(HandoffInitResponse{...})` etc., NOT `Json(ApiResponse{success, data})`). The existing `remote-frontend/src/api.ts:36,53` does `return res.json()` directly — follow that pattern. Define a local `ApiResponse<T>` type (matching the node frontend's envelope shape, used by OTHER hive routes that DO wrap — keep it exported for future use) but do NOT use it in oauth.ts/profile.ts. Remove the `import type { ApiResponse } from 'shared/types'` — instead define a local `ApiResponse<T>` type or inline it (the hive `remote-frontend/` has no `shared/` alias yet — task 201 adds it, but this task runs in phase 1 before 201; use a local type to avoid a cross-phase dependency).
  - **Sibling alignment:** Read `frontend/src/lib/api/utils.ts`. Justify any divergence in the ledger (e.g. local `ApiResponse` type vs `shared/types` import).

- **File:** `remote-frontend/src/lib/api/profile.ts` (CREATE)
  - **Before:** (file does not exist)
  - **After:** `profileApi.get()` → `GET /v1/profile` with header `Authorization: Bearer <localStorage.getItem('access_token')>` (NOT `credentials: include` — the hive auth middleware reads only the Bearer header, `crates/remote/src/auth/middleware.rs:46-53`). Returns `ProfileResponse`. Uses `makeRequest` from `./utils` then `if (!response.ok) throw new ApiError(...); return await response.json() as ProfileResponse;` (bare JSON, NOT `handleApiResponse` — the hive `/v1/profile` returns `Json(ProfileResponse{...})` directly, `crates/remote/src/routes/oauth.rs:199`, NOT an `{success, data}` envelope).

- **File:** `remote-frontend/src/lib/api/oauth.ts` (CREATE)
  - **Before:** (file does not exist)
  - **After:** `oauthApi` namespace with:
    - `init(provider: string, returnTo: string, appChallenge: string)` → POST `/v1/oauth/web/init` body `{ provider, return_to: returnTo, app_challenge: appChallenge }` → returns `{ handoff_id: string, authorize_url: string }`. Use `makeRequest` then `if (!response.ok) throw new ApiError(...); return await response.json() as HandoffInitResponse;` (bare JSON — the hive returns `Json(HandoffInitResponse{...})` directly, `crates/remote/src/routes/oauth.rs:55`, NOT an `{success, data}` envelope; do NOT use `handleApiResponse`).
    - `redeem(handoffId: string, appCode: string, appVerifier: string)` → POST `/v1/oauth/web/redeem` body `{ handoff_id: handoffId, app_code: appCode, app_verifier: appVerifier }` → returns `{ access_token: string, refresh_token: string }`. Same bare-JSON pattern (do NOT use `handleApiResponse`).
    - `logout()` → POST `/v1/oauth/logout` with header `Authorization: Bearer <localStorage.getItem('access_token')>`; on success (or any outcome) clear `localStorage.removeItem('access_token')` (the session is gone client-side regardless of server response). Use `makeRequest` in a try/finally (token cleared in `finally`); do NOT call `handleApiResponse` (logout returns 204 No Content, no body).
  - **Sibling alignment:** Read `frontend/src/lib/api/oauth.ts`. It targets the NODE server (`/api/auth/handoff/init`, `/api/auth/status`, `/api/auth/logout`). The hive client targets the HIVE server (`/v1/oauth/web/init`, `/v1/oauth/web/redeem`, `/v1/oauth/logout`) — different paths, different request bodies (`app_challenge`/`app_verifier` not `code`/`state`), different response bodies (`handoff_id`/`authorize_url`/`access_token`/`refresh_token` not `redirect_url`/`profile`). Record all divergences in the ledger. `status()` is dropped (the hive has no `/oauth/status`; login state is derived from `/v1/profile` fetch success in task 101). ALSO read `remote-frontend/src/api.ts:13-55` — it already implements the correct PKCE handoff contract; this task extracts it into `lib/api/oauth.ts` and re-exports from `api.ts` for backwards compat.

- **File:** `remote-frontend/src/api.ts` (EDIT — absorb)
  - **Anchor:** the existing `initOAuth`, `redeemOAuth`, `HandoffInitResponse`, `HandoffRedeemResponse`, `OAuthProvider` exports.
  - **Before:** existing `remote-frontend/src/api.ts` defines these inline.
  - **After:** re-export them from `./lib/api/oauth` for backwards compat with the invitation pages (which still import from `./api`). Keep `getInvitation` and `acceptInvitation` in `api.ts` (invitation flow is unchanged). This avoids breaking the existing 4 auth-stub pages during the phase 1 transition.

## Allowed moves
- Create `remote-frontend/src/lib/api/{utils,profile,oauth}.ts` and the test file.
- Edit `remote-frontend/src/api.ts` to re-export from `./lib/api/oauth`.
- Read-only reference to `frontend/src/lib/api/{oauth,config,utils}.ts`.

## STOP triggers
- If `crates/utils/src/api/oauth.rs:7-33` request or response bodies differ from the shapes assumed above — STOP and record the actual shape in the ledger; the task's `After` must match the server contract, not the node frontend's shape. (Cross-checked: the shapes above ARE the contract.)
- If `remote-frontend/package.json` lacks `vitest` — STOP; task 100 must run first.

## Manual verification (record in decisions-ledger)
- `cd remote-frontend && npx vitest run src/lib/api/oauth.test.ts` exits 0.
- `cd remote-frontend && npx tsc --noEmit` exits 0.
- `cd remote-frontend && npm run lint` exits 0.

## Done when
`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/lib/api/oauth.test.ts" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 102` exits 0
