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
    localStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    sessionStorage.clear()
    localStorage.clear()
  })

  it('redeems with stored verifier and accepts with URL invitation token', async () => {
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
      expect(redeemOAuth).toHaveBeenCalledWith('handoff-123', 'app-code-456', 'stored-verifier', expect.any(AbortSignal))
      expect(acceptInvitation).toHaveBeenCalledWith('url-token', 'access-123', expect.any(AbortSignal))
    })
    expect(screen.getByText('Invitation accepted!')).toBeInTheDocument()
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(localStorage.getItem('access_token')).toBe('access-123')
  })

  it('shows error when oauthError param is present', async () => {
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    sessionStorage.setItem('invitation_token', 'stored-token')
    render(
      <MemoryRouter initialEntries={['/invitations/url-token/complete?error=access_denied']}>
        <Routes>
          <Route path="/invitations/:token/complete" element={<InvitationCompletePage />} />
        </Routes>
      </MemoryRouter>
    )

    await waitFor(() => {
      expect(screen.getByText('OAuth error: access_denied')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
  })

  it('shows error when stored verifier is missing', async () => {
    sessionStorage.setItem('invitation_token', 'stored-token')
    vi.mocked(redeemOAuth).mockResolvedValue({
      access_token: 'access-123',
      refresh_token: 'refresh-123',
    })

    renderInvitationComplete()

    await waitFor(() => {
      expect(screen.getByText('OAuth session lost. Please try again.')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
  })

  it('shows error when invitation token is missing', async () => {
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    vi.mocked(redeemOAuth).mockResolvedValue({
      access_token: 'access-123',
      refresh_token: 'refresh-123',
    })

    render(
      <MemoryRouter initialEntries={['/invitations/complete?handoff_id=handoff-123&app_code=app-code-456']}>
        <Routes>
          <Route path="/invitations/complete" element={<InvitationCompletePage />} />
        </Routes>
      </MemoryRouter>
    )

    await waitFor(() => {
      expect(screen.getByText('Invitation token lost. Please try again.')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
  })

  it('shows error when redeemOAuth fails', async () => {
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    sessionStorage.setItem('invitation_token', 'stored-token')
    vi.mocked(redeemOAuth).mockRejectedValue(new Error('OAuth redemption failed'))

    renderInvitationComplete()

    await waitFor(() => {
      expect(screen.getByText('OAuth redemption failed')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
  })

  it('shows error when acceptInvitation fails after successful redeem', async () => {
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    sessionStorage.setItem('invitation_token', 'stored-token')
    vi.mocked(redeemOAuth).mockResolvedValue({
      access_token: 'access-123',
      refresh_token: 'refresh-123',
    })
    vi.mocked(acceptInvitation).mockRejectedValue(new Error('Accept failed'))

    renderInvitationComplete()

    await waitFor(() => {
      expect(screen.getByText('Accept failed')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(localStorage.getItem('access_token')).toBeNull()
  })

  it('shows error when handoff_id and app_code are missing', async () => {
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    sessionStorage.setItem('invitation_token', 'stored-token')
    render(
      <MemoryRouter initialEntries={['/invitations/url-token/complete']}>
        <Routes>
          <Route path="/invitations/:token/complete" element={<InvitationCompletePage />} />
        </Routes>
      </MemoryRouter>
    )

    await waitFor(() => {
      expect(screen.getByText('Missing OAuth parameters. Please try the invitation link again.')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
  })
})
