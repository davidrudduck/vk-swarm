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

// The Nodes page (rendered via the authenticated routes below) fetches
// organizations and nodes through these API modules. Without mocking them,
// the queries hit the real (unmocked) `fetch`, which is slow/non-deterministic
// in jsdom and leaves the Nodes page stuck in its loading state depending on
// what ran earlier in the file (test isolation failure, F-2026-07-11-01).
// Mock them the same way sibling suites (e.g. src/pages/Nodes.test.tsx) do so
// the queries resolve deterministically and don't depend on the network.
//
// `NodesPage` (task 309) imports `nodesApi`/`organizationsApi` directly from
// their submodules (`@/lib/api/nodes`, `@/lib/api/organizations`) rather than
// through the `@/lib/api` barrel, so both the barrel and the submodules are
// mocked here to cover every import path in play (barrel used by
// `NodeApiKeySection`, submodules used by `NodesPage` itself).
vi.mock('@/lib/api', () => ({
  nodesApi: {
    list: vi.fn().mockResolvedValue([]),
    listApiKeys: vi.fn().mockResolvedValue([]),
  },
  organizationsApi: {
    list: vi.fn().mockResolvedValue([]),
  },
}))

vi.mock('@/lib/api/nodes', () => ({
  nodesApi: {
    list: vi.fn().mockResolvedValue([]),
    listApiKeys: vi.fn().mockResolvedValue([]),
  },
}))

vi.mock('@/lib/api/organizations', () => ({
  organizationsApi: {
    list: vi.fn().mockResolvedValue([]),
  },
}))

import { useProfile } from '@/components/ProfileProvider'
import { oauthApi } from '@/lib/api/oauth'
import { initOAuth, getInvitation } from '@/api'
import { nodesApi, organizationsApi } from '@/lib/api'
import { nodesApi as nodesApiDirect } from '@/lib/api/nodes'
import { organizationsApi as organizationsApiDirect } from '@/lib/api/organizations'

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
    // vi.resetAllMocks() strips the default implementations configured in the
    // vi.mock('@/lib/api', ...) factory above, so re-establish them here on
    // every test so the Nodes page's queries keep resolving deterministically.
    vi.mocked(nodesApi.list).mockResolvedValue([])
    vi.mocked(organizationsApi.list).mockResolvedValue([])
    vi.mocked(nodesApiDirect.list).mockResolvedValue([])
    vi.mocked(nodesApiDirect.listApiKeys).mockResolvedValue([])
    vi.mocked(organizationsApiDirect.list).mockResolvedValue([])
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

    // The root redirect should navigate to /nodes, which renders `NodesPage`
    // (task 309), whose `NodesView` panel heading is "Hive" (not "Nodes" --
    // the old `pages/Nodes.tsx` heading, now superseded).
    // This is the first test in the file to hit the /nodes route, so it pays the cost of
    // React.lazy() compiling/importing the Nodes page chunk for the first time; under a
    // CPU-contended parallel test run (many files/workers at once) that first-import cost
    // can exceed the default 1000ms waitFor timeout, so give it more headroom here.
    await waitFor(
      () => {
        expect(screen.getByRole('heading', { level: 2, name: 'Hive' })).toBeInTheDocument()
      },
      { timeout: 5000 }
    )
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

    // Should render the NodesView panel heading ("Hive")
    await waitFor(() => {
      expect(screen.getByRole('heading', { level: 2, name: 'Hive' })).toBeInTheDocument()
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

describe('Chrome integration (SC8)', () => {
  function renderWithProfileProvider(initial: string) {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const router = createMemoryRouter(createRoutes(), { initialEntries: [initial] })
    return render(
      <QueryClientProvider client={qc}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )
  }

  it('authed routes render the Chrome Navbar with Board/Nodes/Processes NavTabs', async () => {
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

    renderWithProfileProvider('/nodes')

    await waitFor(
      () => {
        expect(screen.getByRole('button', { name: /Board/ })).toBeTruthy()
        expect(screen.getByRole('button', { name: /Nodes/ })).toBeTruthy()
        expect(screen.getByRole('button', { name: /Processes/ })).toBeTruthy()
      },
      { timeout: 5000 }
    )
  }, 10000)

  it('pre-auth /login does NOT render the Chrome Navbar', async () => {
    vi.mocked(useProfile).mockReturnValue({
      isSignedIn: false,
      isLoaded: true,
      profile: null,
    })

    const { container } = renderWithProfileProvider('/login')

    await waitFor(() => {
      expect(screen.getByText('Welcome')).toBeInTheDocument()
    })
    expect(container.querySelector('nav')).toBeNull()
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
