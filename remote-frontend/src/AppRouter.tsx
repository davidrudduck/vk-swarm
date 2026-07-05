import { createBrowserRouter, RouterProvider, Navigate, useSearchParams } from 'react-router-dom'
import { useEffect, useState, lazy, Suspense } from 'react'
import { useProfile } from '@/components/ProfileProvider'
import { NormalLayout } from '@/components/layout/NormalLayout'
import InvitationPage from './pages/InvitationPage'
import InvitationCompletePage from './pages/InvitationCompletePage'
import NotFoundPage from './pages/NotFoundPage'
import { oauthApi } from '@/lib/api/oauth'
import { retrieveVerifier, clearVerifier } from '@/pkce'
import type { OAuthProvider } from '@/api'
import { generateVerifier, generateChallenge, storeVerifier } from '@/pkce'
import { initOAuth } from '@/api'

const Nodes = lazy(() => import('./pages/Nodes').then(m => ({ default: m.Nodes })))
const TasksBoard = lazy(() => import('./pages/Tasks').then(m => ({ default: m.TasksBoard })))

function RootRedirect() {
  const { isSignedIn, isLoaded } = useProfile()

  if (!isLoaded) {
    return <div className="min-h-screen flex items-center justify-center">Loading...</div>
  }

  return isSignedIn ? <Navigate to="/nodes" replace /> : <Navigate to="/login" replace />
}

function LoginPage() {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleOAuthLogin = async (provider: OAuthProvider) => {
    setLoading(true)
    setError(null)
    try {
      const verifier = generateVerifier()
      const challenge = await generateChallenge(verifier)

      storeVerifier(verifier)

      const appBase = import.meta.env.VITE_APP_BASE_URL || window.location.origin
      const returnTo = `${appBase}/oauth/callback`

      const result = await initOAuth(provider, returnTo, challenge)
      window.location.assign(result.authorize_url)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'OAuth init failed')
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 p-4">
      <div className="w-full max-w-md bg-white shadow rounded-lg p-6 space-y-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Welcome</h1>
          <p className="text-gray-600 mt-1">Sign in to your account</p>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded p-3">
            <p className="text-sm text-red-700">{error}</p>
          </div>
        )}

        <div className="border-t border-gray-200 pt-4 space-y-3">
          <p className="text-sm text-gray-600">Choose a provider to sign in:</p>
          <button
            onClick={() => handleOAuthLogin('github')}
            disabled={loading}
            className="w-full py-3 px-4 bg-gray-900 text-white rounded-lg hover:bg-gray-800 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? 'Signing in...' : 'Sign in with GitHub'}
          </button>
          <button
            onClick={() => handleOAuthLogin('google')}
            disabled={loading}
            className="w-full py-3 px-4 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? 'Signing in...' : 'Sign in with Google'}
          </button>
        </div>
      </div>
    </div>
  )
}

function isSafeReturnTo(url: string): boolean {
  try {
    const parsed = new URL(url, window.location.origin);
    return parsed.origin === window.location.origin;
  } catch {
    return url.startsWith('/');
  }
}

function OAuthCallbackPage() {
  const [searchParams] = useSearchParams()
  const [isRedirecting, setIsRedirecting] = useState(false)

  useEffect(() => {
    const completeOAuth = async () => {
      const handoffId = searchParams.get('handoff_id')
      const appCode = searchParams.get('app_code')
      const oauthError = searchParams.get('error')
      const returnTo = searchParams.get('return_to') || '/nodes'
      const safeReturnTo = isSafeReturnTo(returnTo) ? returnTo : '/nodes'

      if (oauthError) {
        window.location.assign(`/login?error=${encodeURIComponent(`OAuth error: ${oauthError}`)}`)
        return
      }

      if (!handoffId || !appCode) {
        window.location.assign(`/login?error=${encodeURIComponent('Missing OAuth parameters')}`)
        return
      }

      try {
        const appVerifier = retrieveVerifier()
        if (!appVerifier) {
          window.location.assign(`/login?error=${encodeURIComponent('OAuth session lost. Please try again.')}`)
          return
        }

        const { access_token } = await oauthApi.redeem(handoffId, appCode, appVerifier)

        localStorage.setItem('access_token', access_token)
        clearVerifier()

        setIsRedirecting(true)
        window.location.assign(safeReturnTo)
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : 'Failed to complete OAuth'
        window.location.assign(`/login?error=${encodeURIComponent(errorMsg)}`)
        clearVerifier()
      }
    }

    completeOAuth()
  }, [searchParams])

  if (isRedirecting) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="text-gray-600">Redirecting...</div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="text-center">
        <div className="text-gray-600">Processing...</div>
      </div>
    </div>
  )
}



export function createRoutes() {
  return [
    { path: '/', element: <RootRedirect /> },
    { path: '/login', element: <LoginPage /> },
    { path: '/oauth/callback', element: <OAuthCallbackPage /> },
    { path: '/invitations/:token/accept', element: <InvitationPage /> },
    { path: '/invitations/:token/complete', element: <InvitationCompletePage /> },
    {
      element: <NormalLayout />,
      children: [
        { path: '/nodes', element: <Suspense fallback={<div className="p-8">Loading nodes...</div>}><Nodes /></Suspense> },
        { path: '/tasks', element: <Suspense fallback={<div className="p-8">Loading tasks...</div>}><TasksBoard /></Suspense> },
        { path: '*', element: <NotFoundPage /> },
      ],
    },
  ]
}

const router = createBrowserRouter(createRoutes())

export default function AppRouter() {
  return <RouterProvider router={router} />
}
