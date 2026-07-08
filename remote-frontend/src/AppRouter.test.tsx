import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createRoutes } from './AppRouter'

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
}))

import { useProfile } from '@/components/ProfileProvider'
import { oauthApi } from '@/lib/api/oauth'
import { initOAuth } from '@/api'

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
    vi.clearAllMocks()
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

    renderWithRouter('/oauth/callback?handoff_id=handoff-123&app_code=app-code-456&return_to=/nodes')

    await waitFor(() => {
      expect(oauthApi.redeem).toHaveBeenCalledWith('handoff-123', 'app-code-456', 'stored-verifier')
    })
    expect(localStorage.getItem('access_token')).toBe('access-123')
    expect(sessionStorage.getItem('oauth_verifier')).toBeNull()
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

    renderWithRouter('/invitations/test-token/accept')

    // InvitationPage will try to fetch the invitation, so it'll show loading state
    // Check for either the loading text or the page heading
    await waitFor(
      () => {
        const loadingText = screen.queryByText('Loading invitation...')
        const headingText = screen.queryByText("You've been invited")
        expect(loadingText || headingText).toBeTruthy()
      },
      { timeout: 2000 }
    )
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
})
