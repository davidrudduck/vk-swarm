import { useEffect, useMemo, useRef, useState } from 'react'
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
  const [orgSlug, setOrgSlug] = useState<string | null>(null)

  const handoffId = qp.get('handoff_id')
  const appCode = qp.get('app_code')
  const oauthError = qp.get('error')

  const hasRun = useRef(false)

  useEffect(() => {
    if (hasRun.current) return
    hasRun.current = true

    let active = true
    let timer: ReturnType<typeof setTimeout> | undefined

    const completeInvitation = async () => {
      if (oauthError) {
        if (active) setError(`OAuth error: ${oauthError}`)
        clearVerifier()
        clearInvitationToken()
        return
      }

      if (!handoffId || !appCode) {
        if (active) setError('Missing OAuth parameters. Please try the invitation link again.')
        clearVerifier()
        clearInvitationToken()
        return
      }

      try {
        const verifier = retrieveVerifier()
        if (!verifier) {
          if (active) setError('OAuth session lost. Please try again.')
          clearInvitationToken()
          return
        }

        const token = urlToken || retrieveInvitationToken()
        if (!token) {
          if (active) setError('Invitation token lost. Please try again.')
          clearVerifier()
          return
        }

        const { access_token } = await redeemOAuth(
          handoffId,
          appCode,
          verifier
        )

        const result = await acceptInvitation(token, access_token)

        clearVerifier()
        clearInvitationToken()
        localStorage.setItem('access_token', access_token)

        if (!active) return

        setSuccess(true)
        setOrgSlug(result.organization_slug)

        timer = setTimeout(() => {
          const appBase =
            import.meta.env.VITE_APP_BASE_URL || window.location.origin
          window.location.assign(`${appBase}`)
        }, 2000)
      } catch (e) {
        clearVerifier()
        clearInvitationToken()
        if (!active) return
        setError(e instanceof Error ? e.message : 'Failed to complete invitation')
      }
    }

    completeInvitation()
    return () => { active = false; if (timer) clearTimeout(timer) }
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
        body={orgSlug ? `Redirecting to ${orgSlug}...` : 'Redirecting...'}
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
