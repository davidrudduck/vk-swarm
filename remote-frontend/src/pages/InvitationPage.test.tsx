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
    <MemoryRouter initialEntries={['/invitations/invite-token/accept']}>
      <Routes>
        <Route path="/invitations/:token/accept" element={<InvitationPage />} />
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
        expect.stringMatching(/^[0-9a-f]{64}$/),
        expect.any(AbortSignal)
      )
    })
    expect(sessionStorage.getItem('oauth_verifier')).toMatch(/^[A-Za-z0-9_-]+$/)
    expect(sessionStorage.getItem('invitation_token')).toBe('invite-token')
    expect(screen.queryByText(/crypto\.subtle/i)).not.toBeInTheDocument()
    expect(resolveInitOAuth).toBeTypeOf('function')
  })

  it('shows error when getInvitation fails', async () => {
    vi.mocked(getInvitation).mockRejectedValue(new Error('Invalid or expired invitation'))
    stubGetRandomValuesOnlyCrypto()

    renderInvitationAccept()

    await waitFor(() => {
      const errorElements = screen.getAllByText('Invalid or expired invitation')
      expect(errorElements.length).toBeGreaterThan(0)
    })
  })

  it('shows error when initOAuth fails', async () => {
    vi.mocked(getInvitation).mockResolvedValue({
      id: 'invitation-1',
      organization_slug: 'test-org',
      organization_name: 'Test Org',
      role: 'admin',
      expires_at: '2026-08-01T00:00:00Z',
    })
    vi.mocked(initOAuth).mockRejectedValue(new Error('OAuth init failed'))
    stubGetRandomValuesOnlyCrypto()

    renderInvitationAccept()

    fireEvent.click(await screen.findByRole('button', { name: 'Continue with GitHub' }))

    await waitFor(() => {
      expect(screen.getByText('OAuth init failed')).toBeInTheDocument()
    })
    // Verify the invitation card is still visible (not replaced by ErrorCard)
    expect(screen.getByText("You've been invited")).toBeInTheDocument()
    // Verify OAuth buttons are still visible for retry
    expect(screen.getByRole('button', { name: 'Continue with GitHub' })).toBeInTheDocument()
  })
})
