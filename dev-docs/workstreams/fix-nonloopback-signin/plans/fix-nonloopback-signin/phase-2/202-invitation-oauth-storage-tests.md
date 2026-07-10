---
id: "202"
phase: 2
title: Cover non-loopback invitation OAuth and completion storage
status: passed
depends_on: ["201"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/pages/InvitationPage.test.tsx
  - remote-frontend/src/pages/InvitationCompletePage.test.tsx
  - remote-frontend/src/pages/HomePage.tsx
  - remote-frontend/src/pages/Nodes.test.tsx
irreversible: false
scope_test: "remote-frontend/src/pages"
allowed_change: create
covers_criteria: [SC4, SC5, SC7, SC9]
---

## Failing test (write first)

Read `remote-frontend/src/pages/Nodes.test.tsx` first and copy its local render/helper style: colocated test file, `vi.mock(...)` at the top, imports after mocks, no shared test harness dependency.

Create `remote-frontend/src/pages/InvitationPage.test.tsx`:

```tsx
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import InvitationPage from './InvitationPage'

vi.mock('../api', () => ({
  getInvitation: vi.fn(),
  initOAuth: vi.fn(),
}))

import { getInvitation, initOAuth } from '../api'

function stubGetRandomValuesOnlyCrypto() {
  vi.stubGlobal('crypto', {
    getRandomValues: vi.fn((array: Uint8Array) => {
      for (let i = 0; i < array.length; i += 1) {
        array[i] = i + 2
      }
      return array
    }),
  })
}

function renderInvitationAccept() {
  return render(
    <MemoryRouter initialEntries={['/invitations/invite-token' + '/accept']}>
      <Routes>
        <Route path={'/invitations/:token' + '/accept'} element={<InvitationPage />} />
      </Routes>
    </MemoryRouter>
  )
}

describe('InvitationPage OAuth PKCE flow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    sessionStorage.clear()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
    sessionStorage.clear()
  })

  it('starts invitation OAuth with fallback PKCE challenge when crypto.subtle is unavailable', async () => {
    vi.mocked(getInvitation).mockResolvedValue({
      id: 'invitation-1',
      organization_slug: 'test-org',
      organization_name: 'Test Org',
      role: 'admin',
      expires_at: '2026-08-01T00:00:00Z',
    })
    let resolveInitOAuth: ((value: { handoff_id: string; authorize_url: string }) => void) | undefined
    vi.mocked(initOAuth).mockReturnValue(
      new Promise((resolve) => {
        resolveInitOAuth = resolve
      })
    )
    stubGetRandomValuesOnlyCrypto()

    renderInvitationAccept()

    fireEvent.click(await screen.findByRole('button', { name: 'Continue with GitHub' }))

    await waitFor(() => {
      expect(initOAuth).toHaveBeenCalledWith(
        'github',
        expect.stringContaining('/invitations/invite-token/complete'),
        expect.stringMatching(/^[0-9a-f]{64}$/)
      )
    })
    expect(sessionStorage.getItem('oauth_verifier')).toMatch(/^[A-Za-z0-9_-]+$/)
    expect(sessionStorage.getItem('invitation_token')).toBe('invite-token')
    expect(screen.queryByText(/crypto\.subtle/i)).not.toBeInTheDocument()
    expect(resolveInitOAuth).toBeTypeOf('function')
  })
})
```

Create `remote-frontend/src/pages/InvitationCompletePage.test.tsx`:

```tsx
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import InvitationCompletePage from './InvitationCompletePage'

vi.mock('../api', () => ({
  redeemOAuth: vi.fn(),
  acceptInvitation: vi.fn(),
}))

import { acceptInvitation, redeemOAuth } from '../api'

function renderInvitationComplete() {
  return render(
    <MemoryRouter initialEntries={['/invitations/url-token/complete?handoff_id=handoff-123&app_code=app-code-456']}>
      <Routes>
        <Route path="/invitations/:token/complete" element={<InvitationCompletePage />} />
      </Routes>
    </MemoryRouter>
  )
}

describe('InvitationCompletePage storage handoff', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    sessionStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    sessionStorage.clear()
  })

  it('redeems with stored verifier and accepts with stored invitation token', async () => {
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    sessionStorage.setItem('invitation_token', 'stored-token')
    vi.mocked(redeemOAuth).mockResolvedValue({
      access_token: 'access-123',
      refresh_token: 'refresh-123',
    })
    vi.mocked(acceptInvitation).mockResolvedValue({
      organization_id: 'org-1',
      organization_slug: 'test-org',
      role: 'admin',
    })

    renderInvitationComplete()

    await waitFor(() => {
      expect(redeemOAuth).toHaveBeenCalledWith('handoff-123', 'app-code-456', 'stored-verifier')
      expect(acceptInvitation).toHaveBeenCalledWith('stored-token', 'access-123')
    })
    expect(screen.getByText('Invitation accepted!')).toBeInTheDocument()
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
  })
})
```

`InvitationPage.test.tsx` must fail before task 101 because clicking the provider throws before `initOAuth()` when `crypto.subtle` is absent.

## Change

### File: `remote-frontend/src/pages/InvitationPage.test.tsx` (CREATE)

Create exactly the first test file shown in `## Failing test (write first)`.

### File: `remote-frontend/src/pages/InvitationCompletePage.test.tsx` (CREATE)

Create exactly the second test file shown in `## Failing test (write first)`.

### File: `remote-frontend/src/pages/Nodes.test.tsx` (READ-ONLY sibling acknowledgement)

Read this sibling first. Match its colocated test style: top-level `vi.mock(...)`, imports after mocks, local render helper, no global test utility extraction. Record any divergence in `docs/plans/fix-nonloopback-signin/decisions-ledger.md`.

### File: `remote-frontend/src/pages/HomePage.tsx` (READ-ONLY sibling acknowledgement)

Read this sibling because `wai-plan-lint.sh` flags new files beside existing page files. It is a simple page component and not a test pattern sibling; do not edit it. If execution discovers it affects route rendering conventions, record the reason in `docs/plans/fix-nonloopback-signin/decisions-ledger.md`.

## Allowed moves

- Create only the two new invitation test files.
- Read but do not edit `remote-frontend/src/pages/Nodes.test.tsx`.
- Keep mocked `initOAuth()` pending after the challenge/storage assertions so jsdom does not execute
  production navigation. Task 301 performs the real redirect verification over LAN.
- Do not use `vi.spyOn(window.location, 'assign')` or `Object.defineProperty(window, 'location', ...)` unless a fresh repo-local proof shows the descriptor is configurable; if that proof exists, record it in the decisions ledger before using it.
- Do not edit `InvitationPage.tsx`, `InvitationCompletePage.tsx`, `api.ts`, or `pkce.ts` in this task.
- Restore globals, mocks, and session storage after each test.

## STOP triggers

- The tests require production changes to pass after tasks 101 and 201.
- `initOAuth()` receives a non-hex or non-64-character challenge.
- The invitation token is not stored before the OAuth redirect starts.
- A reviewer asks for a jsdom redirect assertion without a safe, repo-proven way to intercept
  `window.location.assign`; keep the unit test focused on OAuth start/storage and rely on task
  301's mandatory LAN browser redirect verification.
- Invitation completion accepts with the URL token when a stored token is present.
- Any unlisted file needs an edit.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npm run test:run -- src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh fix-nonloopback-signin 202` exits 0.
