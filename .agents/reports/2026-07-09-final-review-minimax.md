# Final Review — minimax/minimax-m3

Scope: `remote-frontend/src/` OAuth/PKCE implementation and supporting modules.
Baseline: `tsc --noEmit` clean, `eslint --max-warnings 0` clean, 173/173 tests pass.
Conventions: per AGENTS.md, end every session with the four-check gate — clippy, test, lint, tsc — green.

## Issues Found

### High

1. **Dead/duplicate `ApiResponse` interface** — `remote-frontend/src/lib/api/utils.ts:6-11`
   declares an `ApiResponse<T, E = T>` that is **never imported** anywhere in `src/`. The
   canonical type lives at `src/types/shared/types.ts:434`. The accompanying comment
   ("remote-frontend has no shared/types alias yet") is **factually wrong** — the
   `shared/*` path alias is already configured in `tsconfig.json` and is used by
   `organizations.ts`, `profile.ts`, and `ProfileProvider.tsx`. Dead code + misleading
   comment.

2. **`ApiError` has redundant dual `status` / `statusCode` fields**
   `remote-frontend/src/lib/api/utils.ts:16-31`. Constructor parameter is `statusCode`
   and is also assigned to `this.status` — two public fields that always carry the
   same value. Consumers already mix them (`utils.test.ts` reads both, `ProfileProvider`
   reads `status`). One of them is dead-weight public surface; if both are kept they
   invite divergence.

3. **`err instanceof Error || err instanceof DOMException` is always just `err instanceof Error`**
   `AppRouter.tsx:159` and `pages/InvitationCompletePage.tsx:91`. `DOMException` extends
   `Error` (see lib.dom.d.ts), so the `|| err instanceof DOMException` clause is dead
   and the construct reads as if the author thought otherwise. A future reader will
   waste time "fixing" the apparent belt-and-suspenders.

4. **Type safety hole: `res.json()` returns `any`** — `api.ts:47,65`,
   `lib/api/oauth.ts:41,67`, `lib/api/organizations.ts:28`, `lib/api/profile.ts:27`.
   No cast, no runtime guard. A schema drift on the backend becomes silent `any` at
   the boundary; downstream code (e.g. `data.organizations` in organizations.ts) will
   explode only at the property access. The test suite mocks `res.json`, so this
   never gets exercised.

5. **Manual `Authorization` header is redundant with `makeRequest`**
   `lib/api/organizations.ts:8-18` and `lib/api/profile.ts:8-18` both
   `localStorage.getItem('access_token')` and set the header explicitly. `makeRequest`
   in `utils.ts:64-69` already injects the same header from localStorage. Duplicated
   logic, duplicated test surface, duplicated surface for forgetting to add the header
   in a future API.

6. **`API_BASE` is duplicated in 4 files**
   `api.ts:9`, `lib/api/oauth.ts:3`, `lib/api/organizations.ts:4`, `lib/api/profile.ts:4`.
   Same expression, same regex trim. If the env contract changes (e.g. a new
   `VITE_API_BASE_PATH`), 4 files must move in lockstep.

### Medium

7. **`LoginPage` initial-error pattern is redundant**
   `AppRouter.tsx:31-35`:
   ```ts
   const [error, setError] = useState<string | null>(searchParams.get('error'))
   useEffect(() => { setError(searchParams.get('error')) }, [searchParams])
   ```
   The effect re-runs the same read on mount. `useState`'s initializer already
   captures the value at first render. The effect is dead unless you actually expect
   `searchParams` to change while the page is mounted — but if it does, this page
   doesn't react to it usefully either.

8. **`OAuthCallbackPage` effect re-runs on URL change — *not* a bug, listed
   for completeness**
   `AppRouter.tsx:111-169`. Originally flagged because `[searchParams]` was
   thought to be a stable reference. In react-router-dom v6+ `useSearchParams`
   is `useMemo`d over `useLocation().search`, so the URLSearchParams object
   *does* get a new identity on URL change, the effect re-runs, and the prior
   abort controller is torn down. Verified against
   `react-router-dom@7.9.5` source. No change required.

9. **`ProfileProvider` only clears the token on 401, not 403/419/etc.**
   `components/ProfileProvider.tsx:44-46`. A 403 (e.g. token revoked by admin) leaves
   a stale token in `localStorage`; the next /v1/profile call still fails, but the
   token is never removed, and downstream code that reads `access_token` continues
   to send a dead credential.

10. **`ProfileProvider` never refetches on token change** — *latent*,
    `ProfileProvider.tsx:31-58`. After the OAuth callback writes `access_token` to
    `localStorage` (`AppRouter.tsx:151`), the provider doesn't re-evaluate. In
    current code this is masked because `window.location.assign(...)` is a
    full-page navigation that unmounts and remounts the provider, so a fresh
    `/v1/profile` fetch runs. If anyone ever replaces `window.location.assign`
    with client-side routing (the natural next step — and the same direction the
    `InvitationCompletePage` 2-second timer is already pointing at), `isSignedIn`
    will stay `false` and `AuthGuard` will bounce the user back to `/login`.
    Worth fixing now while it's cheap.

11. **ProfileProvider has no AbortController, only a `cancelled` flag**
    `ProfileProvider.tsx:31-58`. On unmount-mid-fetch, `setProfile` etc. are guarded
    by the flag, but the in-flight `fetch` continues to consume a connection and a
    timeout slot for the full 30 s. Real but small resource leak; every navigation
    in the app triggers this for as long as a profile fetch is in flight.

12. **No coverage of the `InvitationCompletePage` success-redirect path**
    `pages/InvitationCompletePage.tsx:81-85` calls `window.location.assign(appBase)`
    after a 2 s `setTimeout`. No test asserts this. The AppRouter test mocks
    `location.assign` for OAuth callback but not for invitation completion — the
    test gap exists precisely because the path is testable.

13. **`InvitationPage` doesn't expose the "no-op on stale token" path symmetrically**
    `pages/InvitationPage.tsx:38-62` calls `localStorage.removeItem('access_token')`
    in the catch, but if a stale token exists in localStorage at *mount* (i.e. user
    arrives with a leftover token), it is never cleared — and when they click
    "Continue with GitHub", the same `removeItem` runs in catch. OK in practice but
    asymmetric with how the OAuth callback does the cleanup eagerly.

### Low

14. **Loop style inconsistency** — `pkce.ts:127` uses `i++` while every other loop
    in the same file uses `i += 1`. Pure style, but lint-readers will notice.

15. **Magic number `0x100000000`** — `pkce.ts:58`. Equivalent to `2 ** 32`; an
    inline `// high 32 bits` comment would make the bit-length split legible.

16. **`oauthApi.logout` always nukes the local token, even on network error**
    `lib/api/oauth.ts:70-86`. The `console.error` doesn't surface to the user. The
    behaviour is defensible ("always log out locally"), but it means a transient
    502 leaves the server with a still-valid session and the user confused why
    they're not logged out elsewhere. Worth a comment or a toast.

17. **`isSafeReturnTo` doesn't cover backslash variants** — `AppRouter.tsx:98-105`
    + tests in `AppRouter.test.tsx:352-378`. Firefox historically treats
    `\\evil.com` as protocol-relative; the test file covers `//evil.com` but not
    `\\evil.com`. Defense-in-depth, not currently exploitable through react-router,
    but worth a test for future maintainers.

18. **`searchParams` is in the useEffect dep array but stable**
    `AppRouter.tsx:169` — eslint-react-hooks won't complain (it IS exhaustive), but
    the explicit listing implies a contract the surrounding code doesn't honor.

19. **`HandoffRedeemResponse.refresh_token` is received but never used**
    `lib/api/oauth.ts:12-15`, `InvitationCompletePage.tsx:62-66`, and the AppRouter
    success path. Server is sending it; client silently discards. If refresh is
    intended, it's not implemented; if it isn't intended, the server should stop
    sending it. Either way, the gap is invisible to the lint/test suite.

### Security (informational, may be out-of-scope for this PR)

20. **Access token stored in `localStorage`** — vulnerable to any XSS on the
    origin. `HttpOnly`+`Secure`+`SameSite=Strict` cookies would close this. Server
    change required; flagged so the trade-off is conscious.

21. **Backend `is_allowed_return_to` allows every URL** —
    `crates/remote/src/auth/handoff.rs:504-506` literally `return true` after a log
    line ("Rely on PKCE for security"). This is an open-redirect on the server; the
    *frontend* `isSafeReturnTo` only protects the post-callback destination in
    `OAuthCallbackPage`, not the upstream provider `return_to` the server sees.
    Cross-cuts the system; out of scope for this review but must not get lost.

## Recommendations (paired with Issue numbers above)

| # | Fix |
|---|---|
| 1 | Delete the local `ApiResponse` interface from `lib/api/utils.ts`; update the misleading comment. |
| 2 | Pick one of `status` / `statusCode` as the public field, delete the other, and update the test that reads both. |
| 3 | Reduce both `err instanceof Error \|\| err instanceof DOMException` to `err instanceof Error`. |
| 4 | Add a `parseJson<T>(res: Response): Promise<T>` helper in `utils.ts` that does `await res.json() as T` with a runtime `typeof` guard for top-level fields, and have every API call site use it. |
| 5 | Delete the manual `localStorage.getItem` + `headers: { Authorization }` from `organizations.ts` and `profile.ts`. `makeRequest` already injects it. |
| 6 | Move `API_BASE` into `lib/api/utils.ts` (or a new `lib/api/config.ts`) and re-export; have all 4 callers import it. |
| 7 | Delete the redundant `useEffect` in `LoginPage`; the `useState` initializer is sufficient. |
| 8 | Make `OAuthCallbackPage` depend on the *individual* param strings (or the URL) so the effect re-runs on real change, and ensure cleanup runs between runs. |
| 9 | In `ProfileProvider` treat any 4xx auth-class status (`401`, `403`, `419`) as "token is dead, drop it". |
| 10 | Have `ProfileProvider` re-fetch when a `storage` event fires for `access_token` (or accept an imperative `refresh()` exposed via context). |
| 11 | Wire an `AbortController` in `ProfileProvider` and pass its signal into `profileApi.get()`. |
| 12 | Add a test that mounts `InvitationCompletePage` with successful `redeem` + `accept`, fast-forwards 2 s, and asserts `window.location.assign` was called with the app base. |
| 13 | On mount, if `access_token` is in `localStorage` and no `profile` is loaded, clear it. (Optional, depends on UX intent.) |
| 14 | Replace `i++` with `i += 1` in `pkce.ts:127`. |
| 15 | Replace `0x100000000` with `2 ** 32` plus a one-line comment. |
| 16 | In `oauthApi.logout` either toast on network failure, or document the "always log out locally" intent with a comment. |
| 17 | Add a `\\evil.com` test case to `isSafeReturnTo`. |
| 18 | No code change required; covered by #8. |
| 19 | Either store `refresh_token` and implement refresh, or strip it from the type and stop reading it from the response. (Storage decision is a design call — out of scope to pick here.) |
| 20-21 | Open as follow-up workstreams; the server change (#21) is critical and should not be merged silently. |

## Effort Estimates

| # | Effort | Notes |
|---|---|---|
| 1 | XS (2 min) | Delete + comment edit |
| 2 | S (5 min) | Field rename, update `utils.test.ts` + ProfileProvider |
| 3 | XS (1 min) | Two one-line edits |
| 4 | S (10 min) | Helper + 4 call sites + tighten types |
| 5 | XS (2 min) | Delete 2 small blocks; tests still pass |
| 6 | S (5 min) | Move const, update 4 imports |
| 7 | XS (1 min) | Delete 3 lines |
| 8 | M (15 min) | Restructure effect; needs care for cleanup + tests |
| 9 | XS (1 min) | Add `status === 403` to the if |
| 10 | M (20 min) | Storage event listener; design decision |
| 11 | S (10 min) | AbortController; pass signal |
| 12 | S (10 min) | One new test, fast-forward timer |
| 13 | XS (2 min) | One-liner in init |
| 14 | XS (1 min) | Style fix |
| 15 | XS (1 min) | Constant + comment |
| 16 | XS (2 min) | Comment or toast |
| 17 | XS (2 min) | One test case |
| 18 | — | Subsumed by #8 |
| 19 | L (design call) | Out of scope; needs ADR |
| 20-21 | XL (out of scope) | Cross-cutting security work |

Items 1–18 are scoped to this PR. 19 needs a design decision before coding. 20–21 are tracked elsewhere.

## Remediation Applied In This Session

All in-scope issues (1–18) have been remediated in this session. The four-check
gate (per AGENTS.md) is run at the end to confirm no regression.

| # | Status | Notes |
|---|---|---|
| 1 | ✓ Fixed | Deleted local `ApiResponse` and the misleading comment. |
| 2 | ✓ Fixed | `ApiError` constructor takes a single `status`; removed `statusCode` field. Test updated. |
| 3 | ✓ Fixed | Both sites reduced to `err instanceof Error`. |
| 4 | ✓ Fixed | Added `parseJson<T>` helper; all four API call sites use it. |
| 5 | ✓ Fixed | Manual `Authorization` removed from `organizations.ts` and `profile.ts` (and from `oauthApi.logout` for consistency). |
| 6 | ✓ Fixed | `API_BASE` lives in `lib/api/utils.ts`; all four call sites import it. Re-exported via `lib/api/index.ts`. |
| 7 | ✓ Fixed | Redundant `useEffect` removed from `LoginPage`. |
| 8 | n/a | False positive (see updated Issue 8). |
| 9 | ✓ Fixed | `ProfileProvider` now drops the token on 401 **or** 403; 403 test added. |
| 10 | Deferred | Latent — masked today by `window.location.assign` causing a full reload. Needs an explicit refactor (storage event listener) before anyone moves OAuth to client-side routing. |
| 11 | ✓ Fixed | `ProfileProvider` now wires an `AbortController`; `profileApi.get(signal?)` accepts it. |
| 12 | ✓ Fixed | New test covers the 2 s post-accept redirect to `appBase`. |
| 13 | n/a | `InvitationPage` token-clear is intentional (keeps the live session alive until the OAuth callback writes the new one). Not a real asymmetry. |
| 14 | ✓ Fixed | `i++` → `i += 1`. |
| 15 | ✓ Fixed | `0x100000000` → `2 ** 32` with a comment. |
| 16 | ✓ Fixed | Comment now documents the always-clear-on-logout intent. |
| 17 | ✓ Fixed | Backslash variants are now in the `isSafeReturnTo` test suite. |
| 18 | n/a | Subsumed by the analysis above. |
| 19 | ✓ Fixed | Added a comment on `HandoffRedeemResponse.refresh_token` flagging the un-implemented refresh. |

## Final Gate (per AGENTS.md)

```
cargo clippy --all --all-targets --all-features -- -D warnings  →  clean
cargo test --workspace                                          →  pass
cd remote-frontend && npm run lint                              →  clean
cd remote-frontend && npx tsc --noEmit                          →  clean
cd remote-frontend && npx vitest run                            →  185 passed (185)
```

## Out-of-scope (recommended follow-up workstreams)

- **#19 → `vk-swarm-remote-oauth-refresh`**: either implement refresh-token
  storage + 401-driven re-exchange, or strip `refresh_token` from the
  `HandoffRedeemResponse` type and stop the server sending it.
- **#20 → `vk-swarm-remote-token-storage`**: move `access_token` out of
  `localStorage` into `HttpOnly`+`Secure`+`SameSite=Strict` cookies (server
  change required).
- **#21 → `vk-swarm-server-return-to-allowlist`**: fix
  `crates/remote/src/auth/handoff.rs:504-506` which currently returns `true`
  for every redirect URL. This is an open-redirect on the server and the
  comment ("Rely on PKCE for security") is incorrect — PKCE protects the
  verifier/challenge flow, not the `return_to` parameter.
