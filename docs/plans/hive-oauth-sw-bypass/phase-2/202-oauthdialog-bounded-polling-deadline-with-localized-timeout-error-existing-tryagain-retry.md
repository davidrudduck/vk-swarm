---
id: "202"
phase: 2
title: "OAuthDialog: bounded polling deadline with localized timeout error (existing tryAgain = retry)"
status: ready
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/components/dialogs/global/__tests__/OAuthDialog.test.tsx"
  - "frontend/src/components/dialogs/global/OAuthDialog.tsx"
siblings: ["frontend/src/components/dialogs/tasks/__tests__/TaskFormSheet.test.tsx"]
irreversible: false
scope_test: "frontend/src/components/dialogs/global/__tests__/OAuthDialog.test.tsx"
allowed_change: mixed
covers_criteria: []
covers_tests: ["TS2"]
---
## Failing test (write first)
Create `frontend/src/components/dialogs/global/__tests__/OAuthDialog.test.tsx` FIRST and run it (RED: no deadline exists yet, so the timeout assertions fail).

Harness — follow the repo's NiceModal mock precedent in sibling `frontend/src/components/dialogs/tasks/__tests__/TaskFormSheet.test.tsx` (read it first; note the F-2026-07-31-02 trap: never close a vi.mock factory over a hoisted import — define spies INSIDE factories or via vi.hoisted):

- `vi.useFakeTimers()`; wrap EVERY `vi.advanceTimersByTime(...)` in `act()`.
- vi.mock `react-i18next`: `useTranslation: () => ({ t: (k: string) => k })` — assertions match raw keys.
- vi.mock `@ebay/nice-modal-react` following the TaskFormSheet pattern but with the members OAuthDialog uses: `useModal: () => ({ visible: true, resolve: vi.fn(), hide: vi.fn(), remove: vi.fn() })`, `create: (C) => C`, and matching `default` export. Keep references to resolve/hide via vi.hoisted so assertions can reach them.
- vi.mock `@/hooks/auth/useAuthStatus`: factory returns a spy that records its `enabled` argument and returns a mutable module-level result object, initially `{ data: { logged_in: false }, isError: false }` (mutable so the success case can flip it).
- vi.mock `@/hooks/auth/useAuthMutations` with a factory of shape `(options) => ({ initHandoff: { mutate: () => options.onInitSuccess({ authorize_url: 'http://hive.test/v1/oauth/github/start?handoff_id=x' }) } })` — capture the OPTIONS OBJECT the component passes to the hook; `onInitSuccess` is NOT an argument of mutate.
- vi.mock `@/components/ConfigProvider` (`useUserSystem: () => ({ reloadSystem: vi.fn() })`); stub `window.open` returning `{ closed: false, close: vi.fn() }`.
- Render the dialog component directly (the nice-modal mock makes it visible), click the GitHub button to enter waiting + polling.

Assertions:
1. `act(() => vi.advanceTimersByTime(POLL_DEADLINE_MS - 1000))`: waiting state still rendered (`oauth.waitingTitle` present); latest `useAuthStatus` call has `enabled: true`.
2. `act(() => vi.advanceTimersByTime(2000))` (past deadline): error state renders `oauth.timeoutError` AND the existing retry button `oauth.tryAgain`; latest `useAuthStatus` call has `enabled: false` (polling ceased).
3. Clicking the `oauth.tryAgain` button returns the dialog to the provider-select state (`oauth.title` and the provider buttons rendered).
4. Success-before-deadline (fresh render): flip the mutable status result to `{ data: { logged_in: true, profile: { username: 'u' } }, isError: false }` before the deadline; assert `reloadSystem` was called and, after `act(() => vi.advanceTimersByTime(1500))`, `modal.resolve` and `modal.hide` were called — and no timeout error ever rendered.
5. Timer cleanup: in a fresh render enter polling, then `unmount()`; assert `vi.getTimerCount() === 0` (the deadline timer was cleared; it is > 0 while polling before unmount).

Run: `cd frontend && npx vitest run src/components/dialogs/global/__tests__/OAuthDialog.test.tsx` — must FAIL before the component change.


## Change
**File:** `frontend/src/components/dialogs/global/OAuthDialog.tsx`

**Anchor 1:** module scope, after the `OAuthState` type (ends L28).
Add:
```ts
// Bounded window for the whole OAuth round-trip. Without a deadline a dead flow
// polls /api/auth/status forever and presents as a silent spinner (F-2026-08-04-01).
export const POLL_DEADLINE_MS = 120_000;
```

**Anchor 2:** inside the component, after the `useEffect` that handles `isStatusError` (L72-81). Add a deadline effect EXACTLY as follows (deps are `[isPolling]` ONLY — the timer must NOT reset when the i18n `t` identity changes, e.g. on a language switch mid-wait):
```ts
  // Bounded polling: a flow that has not completed within the deadline is dead.
  // eslint-disable-next-line react-hooks/exhaustive-deps -- `t` intentionally
  // omitted: the deadline must not reset on language change; the message is
  // resolved at fire time.
  useEffect(() => {
    if (!isPolling) return;
    const deadline = window.setTimeout(() => {
      setIsPolling(false);
      if (popupRef.current && !popupRef.current.closed) {
        popupRef.current.close();
      }
      setState({ type: 'error', message: t('oauth.timeoutError') });
    }, POLL_DEADLINE_MS);
    return () => window.clearTimeout(deadline);
  }, [isPolling]);
```
(If the repo's eslint config places the disable comment differently for hook deps, match the nearest existing precedent; the dependency array `[isPolling]` is the non-negotiable part.)

**Anchor 3:** the `case 'error':` render branch (L264-289) already renders a retry button labelled `t('oauth.tryAgain')` wired to `handleBack`, plus a close button — make NO change to this branch. The timeout path must simply land in it via the state set in Anchor 2.

No changes to `useAuthStatus`, `useAuthMutations`, popup-open logic, or the success path.

**File:** `frontend/src/components/dialogs/global/__tests__/OAuthDialog.test.tsx` (CREATE) — the failing test above.


## Allowed moves
Only: the exported POLL_DEADLINE_MS constant, the one new deadline useEffect (deps [isPolling]), and the new test file. Do not touch the error render branch, state handling, the polling hook, or other render branches.


## STOP triggers
The stated anchors are not found; the error branch does NOT already contain the tryAgain/handleBack button (re-read; if truly absent, halt and record in ledger); the test cannot enter the waiting state under the prescribed mocks; eslint rejects the disable comment in every placement tried; any need to touch an unlisted file (e.g. useAuthStatus.ts).


## Manual verification (record in decisions-ledger)



## Done when
`WAI_ROOT="$(ls -d ~/.claude/plugins/cache/agent-plugins/wai/[0-9]*/ | sort -V | tail -1)"; WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD="cd frontend && npx vitest run src/components/dialogs/global/__tests__/OAuthDialog.test.tsx" bash "$WAI_ROOT/scripts/task-gate.sh" hive-oauth-sw-bypass 202` exits 0

## Orchestrator amendment 2 (2026-08-06, STOP resolution — eslint directive ban)
`frontend/eslint.config.js:51` bans ALL eslint directive comments (`eslint-comments/no-use`
allow: []) and lint runs `--max-warnings 0`, so the Anchor 2 form above cannot pass lint.
Replace Anchor 2's effect with this structural equivalent (same non-negotiable semantics:
deadline depends only on `isPolling`, never resets on `t` identity change, message resolved
at fire time — now from a ref, so no directive is needed):

```ts
  // Bounded polling: a flow that has not completed within the deadline is dead.
  // The deadline must not reset on language change, so the effect depends only
  // on isPolling and resolves the message from a ref at fire time.
  const tRef = useRef(t);
  useEffect(() => {
    tRef.current = t;
  }, [t]);
  useEffect(() => {
    if (!isPolling) return;
    const deadline = window.setTimeout(() => {
      setIsPolling(false);
      if (popupRef.current && !popupRef.current.closed) {
        popupRef.current.close();
      }
      setState({ type: 'error', message: tRef.current('oauth.timeoutError') });
    }, POLL_DEADLINE_MS);
    return () => window.clearTimeout(deadline);
  }, [isPolling]);
```

(`useRef` is already imported.) `npm run lint` (zero warnings) is added to this task's
verification alongside tsc and the scope test.
