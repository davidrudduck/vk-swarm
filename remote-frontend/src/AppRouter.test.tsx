import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createRoutes, isSafeReturnTo } from './AppRouter'

vi.mock('@/components/ProfileProvider', () => ({
  useProfile: vi.fn(),
  ProfileProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}))

vi.mock('@/lib/api/oauth', () => ({
  oauthApi: {
    init: vi.fn(),
    redeem: vi.fn(),
    logout: vi.fn(),
  },
}))

vi.mock('@/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api')>()),
  initOAuth: vi.fn(),
  getInvitation: vi.fn(),
}))

import { useProfile } from '@/components/ProfileProvider'
import { oauthApi } from '@/lib/api/oauth'
import { initOAuth, getInvitation } from '@/api'

describe('AppRouter', () => {
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

  beforeEach(() => {
    vi.resetAllMocks()
    localStorage.clear()
    sessionStorage.clear()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
    localStorage.clear()
    sessionStorage.clear()
  })

  function renderWithRouter(initialEntry = '/') {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    })
    const router = createMemoryRouter(createRoutes(), {
      initialEntries: [initialEntry],
    })

    return render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )
  }

  it('unauthenticated: hitting / redirects to /login', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })

    renderWithRouter('/')

    // The root redirect should navigate to /login, which renders the LoginPage heading
    await waitFor(() => {
      expect(screen.getByText('Welcome')).toBeInTheDocument()
      expect(screen.getByText('Sign in to your account')).toBeInTheDocument()
    })
  })

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
    sessionStorage.setItem('invitation_token', 'stored-token')
    const mockAssign = vi.fn()
    vi.stubGlobal('location', { ...window.location, assign: mockAssign })

    renderWithRouter('/oauth/callback?handoff_id=handoff-123&app_code=app-code-456&return_to=/nodes')

    await waitFor(() => {
      expect(oauthApi.redeem).toHaveBeenCalledWith('handoff-123', 'app-code-456', 'stored-verifier', expect.any(AbortSignal))
    })
    expect(localStorage.getItem('access_token')).toBe('access-123')
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(mockAssign).toHaveBeenCalledWith('/nodes')
  })

  it('authenticated: hitting / redirects to /nodes', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: true,
      isLoaded: true,
      profile: {
        user_id: 'test-id',
        username: 'testuser',
        email: 'test@example.com',
        providers: [],
      },
    })

    renderWithRouter('/')

    // The root redirect should navigate to /nodes, which renders the Nodes page (h2 "Nodes")
    await waitFor(() => {
      expect(screen.getByRole('heading', { level: 2, name: 'Nodes' })).toBeInTheDocument()
    })
  })

  it('authenticated: hitting /nodes renders the Nodes page with layout', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: true,
      isLoaded: true,
      profile: {
        user_id: 'test-id',
        username: 'testuser',
        email: 'test@example.com',
        providers: [],
      },
    })

    renderWithRouter('/nodes')

    // Should render the Nodes page heading
    await waitFor(() => {
      expect(screen.getByRole('heading', { level: 2, name: 'Nodes' })).toBeInTheDocument()
    })
  })

  it('hitting /invitations/:token/accept renders InvitationPage', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    vi.mocked(getInvitation).mockResolvedValue({
      id: 'test-invitation',
      organization_slug: 'test-org',
      organization_name: 'Test Org',
      role: 'admin',
      expires_at: '2026-08-01T00:00:00Z',
    })

    renderWithRouter('/invitations/test-token/accept')

    await waitFor(() => {
      expect(screen.getByText("You've been invited")).toBeInTheDocument()
    })
  })

  it('unknown path renders NotFoundPage', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: true,
      isLoaded: true,
      profile: {
        user_id: 'test-id',
        username: 'testuser',
        email: 'test@example.com',
        providers: [],
      },
    })

    renderWithRouter('/nonexistent')

    // Should render NotFoundPage with the 404 message
    await waitFor(() => {
      expect(screen.getByText('Page not found')).toBeInTheDocument()
    })
  })

  it('login: shows error when initOAuth fails', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    vi.mocked(initOAuth).mockRejectedValue(new Error('Network error'))
    stubGetRandomValuesOnlyCrypto()
    localStorage.setItem('access_token', 'stale-token')

    renderWithRouter('/login')

    fireEvent.click(await screen.findByRole('button', { name: 'Sign in with GitHub' }))

    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeInTheDocument()
    })
    expect(screen.getByRole('button', { name: 'Sign in with GitHub' })).not.toBeDisabled()
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(localStorage.getItem('access_token')).toBeNull()
  })

  it('login: displays error from URL query parameter', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })

    renderWithRouter('/login?error=OAuth%20error%3A%20access_denied')

    await waitFor(() => {
      expect(screen.getByText('OAuth error: access_denied')).toBeInTheDocument()
    })
  })

  it('oauth callback: handles oauthError param', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    sessionStorage.setItem('invitation_token', 'stored-token')
    localStorage.setItem('access_token', 'stale-token')
    const mockAssign = vi.fn()
    vi.stubGlobal('location', { ...window.location, assign: mockAssign })

    renderWithRouter('/oauth/callback?error=access_denied')

    await waitFor(() => {
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('/login?error='))
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('access_denied'))
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(localStorage.getItem('access_token')).toBeNull()
  })

  it('oauth callback: handles missing handoff_id/app_code', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    sessionStorage.setItem('invitation_token', 'stored-token')
    localStorage.setItem('access_token', 'stale-token')
    const mockAssign = vi.fn()
    vi.stubGlobal('location', { ...window.location, assign: mockAssign })

    renderWithRouter('/oauth/callback')

    await waitFor(() => {
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('/login?error='))
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('Missing%20OAuth%20parameters'))
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(localStorage.getItem('access_token')).toBeNull()
  })

  it('oauth callback: handles missing stored verifier', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    sessionStorage.setItem('invitation_token', 'stored-token')
    localStorage.setItem('access_token', 'stale-token')
    const mockAssign = vi.fn()
    vi.stubGlobal('location', { ...window.location, assign: mockAssign })

    renderWithRouter('/oauth/callback?handoff_id=handoff-123&app_code=app-code-456')

    await waitFor(() => {
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('/login?error='))
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('OAuth%20session%20lost'))
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(localStorage.getItem('access_token')).toBeNull()
  })

  it('oauth callback: handles redeem failure', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })
    vi.mocked(oauthApi.redeem).mockRejectedValue(new Error('Redeem failed'))
    sessionStorage.setItem('oauth_verifier', 'stored-verifier')
    sessionStorage.setItem('invitation_token', 'stored-token')
    const mockAssign = vi.fn()
    vi.stubGlobal('location', { ...window.location, assign: mockAssign })

    renderWithRouter('/oauth/callback?handoff_id=handoff-123&app_code=app-code-456')

    await waitFor(() => {
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('/login?error='))
      expect(mockAssign).toHaveBeenCalledWith(expect.stringContaining('Redeem%20failed'))
    })
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
    expect(sessionStorage.getItem('invitation_token')).toBeNull()
    expect(localStorage.getItem('access_token')).toBeNull()
  })
})

describe('isSafeReturnTo', () => {
  it('accepts relative paths', () => {
    expect(isSafeReturnTo('/nodes')).toBe(true)
    expect(isSafeReturnTo('/invitations/token/complete')).toBe(true)
  })

  it('rejects cross-origin URLs', () => {
    expect(isSafeReturnTo('https://evil.com')).toBe(false)
    expect(isSafeReturnTo('https://evil.com/path')).toBe(false)
  })

  it('rejects protocol-relative URLs', () => {
    expect(isSafeReturnTo('//evil.com')).toBe(false)
    expect(isSafeReturnTo('//evil.com/path')).toBe(false)
  })

  it('rejects javascript: URLs', () => {
    expect(isSafeReturnTo('javascript:alert(1)')).toBe(false)
  })

  it('rejects data: URLs', () => {
    expect(isSafeReturnTo('data:text/html,<script>alert(1)</script>')).toBe(false)
  })

  it('handles empty string', () => {
    expect(isSafeReturnTo('')).toBe(true)
  })
})
