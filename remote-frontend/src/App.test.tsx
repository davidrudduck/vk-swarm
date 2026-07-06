import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, waitFor } from '@testing-library/react'
import { useQuery } from '@tanstack/react-query'
import App from './App'
import { useProfile } from '@/components/ProfileProvider'

vi.mock('sonner', () => ({
  Toaster: () => null,
  toast: { error: vi.fn(), success: vi.fn(), message: vi.fn() },
}))

vi.mock('./AppRouter', () => ({
  default: () => {
    const ProfileProbe = () => {
      const { isSignedIn } = useProfile()
      return <span data-testid="profile-probe">{String(isSignedIn)}</span>
    }

    const QueryProbe = () => {
      const { data } = useQuery({
        queryKey: ['probe'],
        queryFn: async () => 'cached-value',
      })
      return <span data-testid="query-probe">{String(data)}</span>
    }

    return (
      <>
        <ProfileProbe />
        <QueryProbe />
        <a href="/nodes">Nodes</a>
      </>
    )
  },
}))

describe('App root providers', () => {
  beforeEach(() => {
    const store: Record<string, string> = {}
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => store[key] ?? null,
      setItem: (key: string, value: string) => {
        store[key] = value
      },
      removeItem: (key: string) => {
        delete store[key]
      },
      clear: () => {
        Object.keys(store).forEach(key => delete store[key])
      },
      key: (index: number) => Object.keys(store)[index] ?? null,
      length: Object.keys(store).length,
    } as Storage)

    localStorage.setItem('access_token', 'test-token')

    vi.stubGlobal('fetch', vi.fn(async (url: string) => {
      if (url.includes('/v1/profile')) {
        return {
          ok: true,
          status: 200,
          json: async () => ({
            user_id: 'u1',
            username: 'alice',
            email: 'a@b.c',
            providers: [],
          }),
        }
      }
      return { ok: false, status: 404, json: async () => ({}) }
    }))
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('provides ProfileProvider context (isSignedIn true after /profile 200)', async () => {
    const { getByTestId } = render(<App />)
    await waitFor(() => {
      const profileProbe = getByTestId('profile-probe')
      expect(profileProbe.textContent).toBe('true')
    })
  })

  it('provides QueryClientProvider context (useQuery consumer returns cached data)', async () => {
    const { getByTestId } = render(<App />)
    await waitFor(() => {
      const queryProbe = getByTestId('query-probe')
      expect(queryProbe.textContent).toBe('cached-value')
    })
  })

  it('renders router outlet (/nodes link is present)', () => {
    const { getByRole } = render(<App />)
    const link = getByRole('link', { name: /Nodes/i })
    expect(link).toBeDefined()
    expect(link.getAttribute('href')).toBe('/nodes')
  })
})
