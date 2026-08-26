# Code Review — Round 2

**Target:** gentle-mongoose   **Range:** `1f2caaea..6a6cf0bf`   **Effort:** high

Round-1 remediations in `6a6cf0bf` verified: XSS escape + test (oauth.rs:355-405, browser_auth_routes.rs:109-130); node-cache start inside epoch fence (login.rs:113-139); non-JSON 401 discriminator (utils.ts:143-148); navbar Sign out always rendered (Navbar.tsx:310-316); popup-null / poll try-catch / startLogin catch (AuthBoundary.tsx:76-113); Probe child (AuthBoundary.test.tsx:61-91); prettier@3.6.1 on the five UI files.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `frontend/src/components/auth/__tests__/AuthBoundary.test.tsx:192` | medium | quality | `npx prettier@3.6.1 --check` fails on this remediation-touched test (`getState.mock.calls.length === 2 ? authorized : unauthorized` wants wrapping). `frontend` `format:check` / `npm run check` will fail. | high | yes |
| 2 | `frontend/src/components/auth/AuthBoundary.tsx:76-79` | medium | correctness | Blocked `window.open` sets `popupRef.current = null` and returns without leaving the prior popup alone. A re-click that is blocked after a successful open overwrites the live popup ref, so `closed` stays false and the interval cannot observe the first popup closing. Do not clobber `popupRef` on a blocked open; just return. | high | yes |
| 3 | `crates/server/tests/browser_auth_routes.rs:45-66` | medium | quality | Frontend 401 discriminator depends on browser-session 401 being non-JSON (`session.rs:67` bare `StatusCode::UNAUTHORIZED`). `protected_api_is_denied_by_default` asserts status 401 only. A future JSON 401 on that path would pass the Rust suite and silently disable teardown. Pin content-type is not `application/json`. | high | yes |
| 4 | `frontend/src/components/auth/AuthBoundary.tsx:111-113` | low | quality | Outer `startLogin` try/catch is untested; every test uses `mockResolvedValue` or a pending promise. Nothing pins Hive-down click behaviour. | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| N1–N14 | ledger `## Post-review known issues` | — | — | Round-1 non-actionable set. | high | already adjudicated; not re-litigated |
| N15 | `frontend/node_modules/prettier` 3.7.3 vs lock 3.6.1 | low | quality | Worktree install drift. `pnpm-lock.yaml` still pins 3.6.1. | high | environment, not this diff; frozen-lockfile install is the fix |

## Verdict: Request changes

Actionable: [1,2,3,4]
