import { useEffect, useMemo, useState } from 'react'
import { useLocation, useParams } from 'react-router-dom'
import { redeemOAuth, acceptInvitation } from '../api'
import {
  retrieveVerifier,
  retrieveInvitationToken,
  clearVerifier,
  clearInvitationToken,
} from '../pkce'

export default function InvitationCompletePage() {
  const { token: urlToken } = useParams()
  const { search } = useLocation()
  const qp = useMemo(() => new URLSearchParams(search), [search])
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  const handoffId = qp.get('handoff_id')
  const appCode = qp.get('app_code')
  const oauthError = qp.get('error')

  useEffect(() => {
    const abortController = new AbortController()
    let timer: ReturnType<typeof setTimeout> | undefined

    const completeInvitation = async () => {
      if (oauthError) {
        if (!abortController.signal.aborted) setError(`OAuth error: ${oauthError}`)
        clearVerifier()
        clearInvitationToken()
        localStorage.removeItem('access_token')
        return
      }

      if (!handoffId || !appCode) {
        if (!abortController.signal.aborted) setError('Missing OAuth parameters. Please try the invitation link again.')
        clearVerifier()
        clearInvitationToken()
        localStorage.removeItem('access_token')
        return
      }

      try {
        const verifier = retrieveVerifier()
        if (!verifier) {
          if (!abortController.signal.aborted) setError('OAuth session lost. Please try again.')
          clearVerifier()
          clearInvitationToken()
          localStorage.removeItem('access_token')
          return
        }

        const token = urlToken || retrieveInvitationToken()
        if (!token) {
          if (!abortController.signal.aborted) setError('Invitation token lost. Please try again.')
          clearVerifier()
          clearInvitationToken()
          localStorage.removeItem('access_token')
          return
        }

        const { access_token } = await redeemOAuth(
          handoffId,
          appCode,
          verifier,
          abortController.signal
        )

        if (abortController.signal.aborted) return

        await acceptInvitation(token, access_token, abortController.signal)

        if (abortController.signal.aborted) return

        clearVerifier()
        clearInvitationToken()
        localStorage.setItem('access_token', access_token)

        setSuccess(true)

        timer = setTimeout(() => {
          const appBase =
            import.meta.env.VITE_APP_BASE_URL || window.location.origin
          window.location.assign(`${appBase}`)
        }, 2000)
      } catch (e) {
        if (abortController.signal.aborted) return
        clearVerifier()
        clearInvitationToken()
        localStorage.removeItem('access_token')
        const message = e instanceof Error || e instanceof DOMException ? e.message : 'Failed to complete invitation'
        setError(message)
      }
    }

    completeInvitation()
    return () => {
      abortController.abort()
      if (timer) clearTimeout(timer)
    }
  }, [handoffId, appCode, oauthError, urlToken])

  if (error) {
    return (
      <StatusCard
        title="Could not accept invitation"
        body={error}
        isError
      />
    )
  }

  if (success) {
    return (
      <StatusCard
        title="Invitation accepted!"
        body="Redirecting..."
        isSuccess
      />
    )
  }

  return (
    <StatusCard
      title="Completing invitation..."
      body="Processing OAuth callback..."
    />
  )
}

function StatusCard({
  title,
  body,
  isError = false,
  isSuccess = false,
}: {
  title: string
  body: string
  isError?: boolean
  isSuccess?: boolean
}) {
  return (
    <div className="min-h-screen grid place-items-center bg-gray-50 p-4">
      <div className="max-w-md w-full bg-white shadow rounded-lg p-6">
        <h2
          className={`text-lg font-semibold ${isError
            ? 'text-red-600'
            : isSuccess
              ? 'text-green-600'
              : 'text-gray-900'
            }`}
        >
          {title}
        </h2>
        <p className="text-gray-600 mt-2">{body}</p>
        {isSuccess && (
          <div className="mt-4 flex items-center text-sm text-gray-500">
            <svg
              className="animate-spin h-4 w-4 mr-2"
              viewBox="0 0 24 24"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
                fill="none"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            Redirecting...
          </div>
        )}
      </div>
    </div>
  )
}
