---
id: "201"
phase: 2
title: Cover non-loopback normal login and callback storage
status: passed
depends_on: ["101"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/AppRouter.test.tsx
irreversible: false
scope_test: "remote-frontend/src/AppRouter.test.tsx"
allowed_change: edit
covers_criteria: [SC2, SC3, SC5, SC6, SC9]
---

## Failing test (write first)

Edit `remote-frontend/src/AppRouter.test.tsx` to add tests for the normal login and callback paths.

Add `fireEvent` to the existing testing-library import:

```ts
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
```

Extend the existing `@/lib/api/oauth` mock and import the mocked API:

```ts
import { oauthApi } from '@/lib/api/oauth'
import { initOAuth } from '@/api'

vi.mock('@/api', () => ({
  initOAuth: vi.fn(),
}))
```

Add these helpers inside `describe('AppRouter', () => {` before `beforeEach`:

```ts
  function stubGetRandomValuesOnlyCrypto() {
    vi.stubGlobal('crypto', {
      getRandomValues: vi.fn((array: Uint8Array) => {
        for (let i = 0; i < array.length; i += 1) {
          array[i] = i + 1
        }
        return array
      }),
    })
  }
```

Extend `beforeEach`:

```ts
    localStorage.clear()
    sessionStorage.clear()
```

Add `afterEach` and import it from `vitest`:

```ts
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
    localStorage.clear()
    sessionStorage.clear()
  })
```

Add these tests after the unauthenticated redirect test:

```ts
  it('login: starts OAuth with fallback PKCE challenge when crypto.subtle is unavailable', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    let resolveInitOAuth: ((value: { handoff_id: string; authorize_url: string }) => void) | undefined
    vi.mocked(initOAuth).mockReturnValue(
      new Promise((resolve) => {
        resolveInitOAuth = resolve
      })
    )
    stubGetRandomValuesOnlyCrypto()

    renderWithRouter('/login')

    fireEvent.click(await screen.findByRole('button', { name: 'Sign in with GitHub' }))

    await waitFor(() => {
      expect(initOAuth).toHaveBeenCalledWith(
        'github',
        expect.stringContaining('/oauth/callback?return_to=%2Fnodes'),
        expect.stringMatching(/^[0-9a-f]{64}$/)
      )
    })
    expect(sessionStorage.getItem('oauth_verifier')).toMatch(/^[A-Za-z0-9_-]+$/)
    expect(screen.queryByText(/crypto\.subtle/i)).not.toBeInTheDocument()
    expect(resolveInitOAuth).toBeTypeOf('function')
  })

  it('oauth callback: redeems with the stored verifier and clears it', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    vi.mocked(oauthApi.redeem).mockResolvedValue({
      access_token: 'access-123',
      refresh_token: 'refresh-123',
    })
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')

    renderWithRouter('/oauth/callback?handoff_id=handoff-123&app_code=app-code-456&return_to=/nodes')

    await waitFor(() => {
      expect(oauthApi.redeem).toHaveBeenCalledWith('handoff-123', 'app-code-456', 'stored-verifier')
    })
    expect(localStorage.getItem('access_token')).toBe('access-123')
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
  })
```

These tests must fail before task 101 because the click path throws before `initOAuth()` when `crypto.subtle` is absent.

## Change

### File: `remote-frontend/src/AppRouter.test.tsx`

- **Anchor:** import block at lines 1-5.
- **Before:**
  ```ts
  import { describe, it, expect, vi, beforeEach } from 'vitest'
  import { render, screen, waitFor } from '@testing-library/react'
  ```
- **After:**
  ```ts
  import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
  import { render, screen, waitFor, fireEvent } from '@testing-library/react'
  ```

- **Anchor:** mock block after `vi.mock('@/lib/api/oauth', ...)`.
- **Before:**
  ```ts
  import { useProfile } from '@/components/ProfileProvider'
  ```
- **After:**
  ```ts
  vi.mock('@/api', () => ({
    initOAuth: vi.fn(),
  }))

  import { useProfile } from '@/components/ProfileProvider'
  import { oauthApi } from '@/lib/api/oauth'
  import { initOAuth } from '@/api'
  ```

- **Anchor:** inside `describe('AppRouter', () => {`, before the existing `beforeEach`.
- **Before:**
  ```ts
  describe('AppRouter', () => {
    beforeEach(() => {
      vi.clearAllMocks()
    })
  ```
- **After:** insert the helper and expanded setup from `## Failing test (write first)`. Do not mock
  `window.location.assign` in this task; modern jsdom may expose it as non-configurable. The login
  click test keeps `initOAuth()` pending after the challenge/storage assertions so production
  navigation is not executed in jsdom. Task 301 performs the real redirect verification over LAN.

- **Anchor:** after the unauthenticated redirect test.
- **Before:** the next test starts with `it('authenticated: hitting / redirects to /nodes', async () => {`.
- **After:** insert the two tests from `## Failing test (write first)` before that existing test.

## Allowed moves

- Edit only `remote-frontend/src/AppRouter.test.tsx`.
- Add the `@/api` mock for `initOAuth`; keep the existing `@/lib/api/oauth` mock for callback redemption.
- Restore globals, storage, and mocks after each test.
- Do not use `vi.spyOn(window.location, 'assign')` or `Object.defineProperty(window, 'location', ...)` unless a fresh repo-local proof shows the descriptor is configurable; if that proof exists, record it in the decisions ledger before using it.
- Do not edit `remote-frontend/src/AppRouter.tsx` or production code in this task.
- Do not change callback route behavior or storage keys.

## STOP triggers

- A reviewer asks for a jsdom redirect assertion without a safe, repo-proven way to intercept `window.location.assign`; keep the unit test focused on OAuth start/storage and rely on task 301's mandatory LAN browser redirect verification.
- The test requires importing `@testing-library/user-event`; do not add a dependency. Use `fireEvent`.
- The OAuth callback test would need production code changes.
- Any unlisted file needs an edit.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npm run test:run -- src/AppRouter.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh fix-nonloopback-signin 201` exits 0.
