import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
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

import { useProfile } from '@/components/ProfileProvider'

describe('AppRouter', () => {
  beforeEach(() => {
    vi.clearAllMocks()
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
