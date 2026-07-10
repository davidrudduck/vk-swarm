# Final Review — moonshotai/kimi-k2.7-code

## Issues Found

1. **Unmounted-component state updates during OAuth handoff (Medium)**
   - `LoginPage` and `InvitationPage` async handlers can call `setState` after the component unmounts if the user navigates away between button click and `initOAuth` resolution.
   - `OAuthCallbackPage` calls `setIsRedirecting(true)` immediately before `window.location.assign(...)`, creating a state update that is wasted right before synchronous navigation and can trigger React warnings.

2. **OAuth init calls are not cancellable (Medium)**
   - `LoginPage` and `InvitationPage` do not pass an `AbortSignal` to `initOAuth`. If those components unmount while waiting for the network, the request continues and may later set state on an unmounted component (see issue 1).

3. **API error wrappers discard backend diagnostics (Medium)**
   - `oauthApi.init`, `oauthApi.redeem`, `getInvitation`, and `acceptInvitation` all throw status-only messages such as `oauth init failed: 400` / `Failed to accept invitation (410)`. The backend sends structured bodies like `{ error: "invalid_app_verifier" }` or `{ message: "Invitation has expired" }` that are never surfaced, making OAuth/invitation failure UIs less useful and tests unable to assert the real reason.

4. **`isSafeReturnTo` allows auth-loop destinations (Medium)**
   - It checks same-origin only, so `/oauth/callback`, `/login`, `/invitations/:token/complete`, and the empty string are all considered safe. A crafted `/login?return_to=/oauth/callback?error=access_denied` can redirect back into the callback handler after the verifier has already been cleared, causing a confusing login/callback loop. It also accepts empty string, which would reload the current page.

5. **`anySignal` leaks event listeners (Low)**
   - When `anySignal` combines several `AbortSignals`, it attaches `abort` listeners to the inputs but only removes the one that fires. If the combined signal aborts because one input fired, listeners on the other inputs remain attached until those inputs themselves abort.

6. **`makeRequest` does not ask for JSON responses (Low)**
   - Default headers set `Content-Type: application/json` but not `Accept: application/json`. This leaves ambiguity if the API ever returns a non-JSON content type for a JSON-shaped endpoint.

7. **Missing unit-test coverage for timeout and signal abort (Low)**
   - `makeRequest` has tests for successful headers and signals, but not for the 30-second timeout firing or for an external `AbortSignal` cancelling a request in flight. There are also no tests asserting that backend error bodies are propagated.

## Recommendations

1. **Guard async lifecycle handlers**
   - Use unmount-time `AbortController` cleanup in `LoginPage` and `InvitationPage`. Pass the controller signal to `initOAuth`. In the catch block, short-circuit if the signal aborted so state is never set on an unmounted component.
   - Remove `setIsRedirecting` from `OAuthCallbackPage`; navigation already unmounts the page, so the extra state update is needless. Let the page continue showing the "Processing…" UI.

2. **Parse and propagate backend error messages**
   - Add a small `formatErrorMessage(response, fallback)` helper in `lib/api/utils.ts` that safely reads `error` or `message` from a non-OK response body (using `response.clone()` to preserve the original).
   - Use it in `oauthApi.init`, `oauthApi.redeem`, `getInvitation`, and `acceptInvitation` so users and tests see actionable messages.

3. **Harden `isSafeReturnTo`**
   - Reject empty values.
   - After confirming same-origin, block paths that are part of the OAuth/invitation machinery: `/oauth/callback`, `/login`, and `/invitations/`.
   - In the `catch` fallback, also require a leading `/` and reject `//` and the blocked prefixes.

4. **Clean up `anySignal` listeners**
   - Store handler references in a `Map`, remove all remaining listeners as soon as one input aborts or the combined signal fires.

5. **Add `Accept: application/json` default header**
   - In `makeRequest`, set `Accept: application/json` when it is not already provided.

6. **Backfill missing tests**
   - Add tests for `makeRequest` timeout firing and for an external `AbortSignal` cancelling an in-flight request.
   - Add tests verifying `oauthApi.init` / `redeem` and `acceptInvitation` surface backend `error`/`message` values.
   - Update `isSafeReturnTo` tests to assert rejected auth-loop paths.

## Effort Estimates

| # | Fix | Effort |
|---|-----|--------|
| 1 | AbortController cleanup + unmount guards in login/invitation pages, remove redirect state in callback | 30 min |
| 2 | Add `formatErrorMessage` helper and use in OAuth/invitation wrappers | 20 min |
| 3 | Harden `isSafeReturnTo` and update tests | 20 min |
| 4 | Clean up `anySignal` listeners | 15 min |
| 5 | Add `Accept: application/json` default header | 5 min |
| 6 | Backfill timeout/signal/error-message unit tests | 30 min |
| **Total** | | **~2 h** |
