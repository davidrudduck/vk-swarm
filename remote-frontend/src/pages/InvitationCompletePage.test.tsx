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
