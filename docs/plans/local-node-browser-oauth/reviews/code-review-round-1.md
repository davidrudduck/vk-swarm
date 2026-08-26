# Code Review — Round 1

**Target:** gentle-mongoose   **Range:** `1f2caaea..d6ccdf07`   **Effort:** high

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `crates/server/src/routes/oauth.rs:138-142,358-363,389` | high | correctness | Public `/api/auth/handoff/complete` reflects attacker-controlled `query.error` into `simple_html_response` (`text/html`) with no HTML escape. `close_window_response` interpolates `message` the same way. A crafted top-level GET executes script on the node origin against the victim's HttpOnly session cookie. | high | yes |
| 2 | `crates/server/src/auth/login.rs:113-136`, `crates/server/src/routes/oauth.rs:222-223,268-314` | high | correctness | Fenced login drops `browser_auth_epoch` before `start_node_cache_sync()`. Disconnect bumps the epoch, shuts the previous node-cache handle, then awaits remote logout. Login can spawn a replacement after that shutdown; disconnect does not shut the replacement. | high | yes |
| 3 | `frontend/src/lib/api/utils.ts:140` | high | correctness | Any HTTP 401 calls `notifyUnauthorized()`, which `AuthBoundary` treats as "this browser session is dead". Browser-session 401 from `require_browser_session` is a bare empty `401`. Hive/proxy 401s are JSON (`RemoteClientError::Auth`, forwarded upstream). Status-only matching is the same class of bug the 503 path already documents as insufficient. Discriminator that works: notify only when 401 is not `application/json`. | high | yes |
| 4 | `frontend/src/components/layout/Navbar.tsx:137,323-335` | medium | correctness | Navbar "Sign out" is gated on node hive `loginStatus`. `AuthBoundary` only mounts Navbar when the browser is authorized. Hive lapse (`local-deployment` clears hive creds, not browser sessions) hides Sign out and shows Sign in, so the user cannot revoke their own browser session. | high | yes |
| 5 | `frontend/src/components/auth/AuthBoundary.tsx:70-99` | medium | correctness | `window.open` null (blocked) leaves `popupRef` null so `closed` is always false and the 10-minute poll runs. `poll()` awaits `getState` with no try/catch, so a rejection skips `stopPolling` on closed. `startLogin` itself is uncaught if `startLogin` throws. | high | yes |
| 6 | `frontend/src/components/auth/__tests__/AuthBoundary.test.tsx:61-84` | medium | quality | "public auth state only" uses one-arg `toHaveBeenCalledWith`, which never matches `makeRequest`'s two-arg `fetch`. Children is a string so no privileged fetch runs anyway. The axis is unguarded. | high | yes |
| 7 | `frontend/src/components/ui/{badge,button,toggle-group,input,progress}.tsx` | medium | quality | `20b8ee3d` reformatted these files with worktree `prettier@3.7.3` while `pnpm-lock.yaml` pins `3.6.1`. `npx prettier@3.6.1 --check` warns on those five; `App.tsx` / `AuthBoundary.tsx` are clean. A frozen-lockfile `format:check` fails. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| N1 | `crates/server/src/auth/cookies.rs:22` | low | correctness | `read_cookie` uses `headers.get(COOKIE)` (first Cookie header only). HTTP/2 may split Cookie headers; later-header session cookies 401. Fail-closed. | high | fail-closed robustness; not a confidentiality hole; out of the production-auth golden path for this close |
| N2 | `crates/server/tests/stream_auth.rs:78` | low | quality | `browser_session_wins_over_an_irrelevant_bad_token_on_direct_streams` calls `configured_with_node_auth` without `ConnectionSecretEnvGuard`; secret leaks for the rest of the binary. Sibling tests hold the guard. | high | test-hygiene; current assertions are secret-agnostic |
| N3 | `crates/server/src/middleware/model_loaders.rs:268-270` | low | quality | Doc still says rejection depends on the connection-token validator being enabled; line 284 rejects token-less non-browser requests unconditionally. | high | stale comment only; fail-closed behaviour is correct |
| N4 | `crates/services/src/services/connection_token.rs:135-148` | low | quality | `validate_for_execution` has zero production callers after this range (wildcard `execution_process_id: None`). Only its unit tests call it. | high | dead API on the type; widening-risk, not a live hole |
| N5 | `crates/server/src/routes/logs.rs:209-213` (and other upgrade sites) | low | correctness | Established WS/SSE streams keep flowing after browser logout. Auth is evaluated once, pre-upgrade. | high | spec TS5 assigns teardown to the client; SC9 requires streams to survive Hive outages |
| N6 | `crates/server/tests/stream_auth.rs:10` | low | quality | Authorized browser + nonexistent attempt `/diff/ws` returns 500 not 404 (loader marks RemoteAttemptNeeded). | high | pre-existing loader semantics, codified in the census |
| N7 | `frontend/src/pages/settings/SwarmSettings.tsx:56,199,205,216` | low | quality | `disconnectConfirm` / `disconnectTitle` / `disconnectHelper` / `disconnectAction` missing from `en/settings.json`; inline defaults work. ja/ko/es already incomplete for swarm keys. | high | cosmetic i18n; inline defaults are the live copy |
| N8 | `frontend/src/components/layout/__tests__/NavbarAuthActions.test.tsx:56-60,158-160` | low | quality | t-mock `defaultValue ?? key` means the `'EVERY browser'` assertion goes dead if the catalog key is added. | high | cosmetic with N7; do not add the key without rewriting the mock |
| N9 | `frontend/src/lib/api/images.ts:17,39`, `backups.ts:53`, 7 WS sites | low | quality | Raw `fetch` / WebSocket paths bypass `notifyUnauthorized`. Task 013 authenticates node-local streams before upgrade. | high | detection gap, not an authorization hole; narrowing 401 (finding 3) must land first |
| N10 | `crates/server/src/auth/node_token.rs:55-68` | low | correctness | Bearer scheme accepts only `"Bearer "` / `"bearer "`, not `BEARER`. RFC 7235 is case-insensitive. | high | no known client sends `BEARER`; not introduced as a live break |
| N11 | `crates/deployment/src/lib.rs:107` | low | quality | `Deployment::spawn_remote_sync` is now a trait method with only test callers; production uses `install_remote_sync`. | high | test seam / public trait default; clippy will not flag it |
| N12 | `crates/services/src/services/node_cache.rs:302-360` | low | quality | Biased `shutdown_rx` can cancel `do_sync` mid-flight; no test proves partial-sync consistency. Self-heals on the next pass. | medium | test-gap; low user impact |
| N13 | `crates/remote/src/routes/tasks.rs:822` | low | correctness | Still passes `assignment.local_attempt_id` into `generate`'s `execution_process_id`. Unmodified in this range. | high | pre-existing, other endpoint, empty `git diff` vs main |
| N14 | TS7 A3 / B3 / B5 / login-shell / cookie Secure | low | — | Workspace flake `F-2026-08-04-02`; live logs/SSE not watched in B3; second Hive account unavailable; login-shell UX locked; trusted-LAN cookies have no `Secure`. | high | already disclosed in README / ledger; do not re-litigate |

## Verdict: Request changes

Actionable: [1,2,3,4,5,6,7]
