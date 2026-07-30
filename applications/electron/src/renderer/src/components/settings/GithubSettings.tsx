import { useEffect, useState } from 'react'
import { has } from '@ryandur/sand'
import { useSyncStatus } from '@state'
import { describeHttpError } from '@transport/http'
import {
  type GithubConnectionStatus,
  deleteGithubConnection,
  getGithubConnection,
  putGithubConnection
} from '@transport/githubConnectionClient'
import ConnectField from './ConnectField'
import { presentSyncError } from './syncErrorCopy'
import {
  type FieldErrors,
  type FieldName,
  hasErrors,
  validateConnectForm
} from './githubConnectForm'
import './GithubSettings.css'

type Props = {
  apiBase: string | undefined
  authToken: string | undefined
  ownerNonce: string | undefined
  onClose: () => void
  open: boolean
}

type View =
  | { phase: 'loading' }
  | { phase: 'settled'; status: GithubConnectionStatus }
  | { phase: 'failed'; message: string }

const GithubSettings = ({
  apiBase,
  authToken,
  ownerNonce,
  onClose,
  open
}: Props): React.JSX.Element => {
  const [view, setView] = useState<View>({ phase: 'loading' })
  const [owner, setOwner] = useState('')
  const [repo, setRepo] = useState('')
  const [token, setToken] = useState('')
  const [enterprise, setEnterprise] = useState(false)
  const [baseUrl, setBaseUrl] = useState('')
  const [busy, setBusy] = useState(false)
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({})

  const edit =
    (field: FieldName, set: (value: string) => void) =>
    (value: string): void => {
      setFieldErrors((previous) => ({ ...previous, [field]: undefined }))
      set(value)
    }

  useEffect(() => {
    if (!open || apiBase === undefined) return
    return getGithubConnection({ apiBase, authToken, ownerNonce })
      .onSuccess((status) => setView({ phase: 'settled', status }))
      .onFailure((error) => setView({ phase: 'failed', message: describeHttpError(error) })).cancel
  }, [open, apiBase, authToken, ownerNonce])

  const settle = (status: GithubConnectionStatus): void => {
    setView({ phase: 'settled', status })
    setToken('')
  }

  const fail = (message: string): void => setView({ phase: 'failed', message })

  const connect = (event: React.FormEvent): void => {
    event.preventDefault()
    const errors = validateConnectForm({ owner, repo, token, enterprise, baseUrl })
    setFieldErrors(errors)
    if (hasErrors(errors)) return
    if (apiBase === undefined) return
    putGithubConnection(
      { apiBase, authToken, ownerNonce },
      enterprise ? { owner, repo, token, base_url: baseUrl } : { owner, repo, token }
    )
      .onPending(setBusy)
      .onSuccess(settle)
      .onFailure((error) => fail(describeHttpError(error)))
  }

  const disconnect = (): void => {
    if (apiBase === undefined) return
    deleteGithubConnection({ apiBase, authToken, ownerNonce })
      .onPending(setBusy)
      .onSuccess(settle)
      .onFailure((error) => fail(describeHttpError(error)))
  }

  const connected = view.phase === 'settled' && view.status.state === 'connected'
  const syncError = useSyncStatus()?.last_error

  return (
    <dialog className="github-settings" open={open} aria-label="GitHub settings">
      <header className="github-settings-header">
        <h2>GitHub</h2>
        <button type="button" aria-label="Close settings" onClick={onClose}>
          ×
        </button>
      </header>
      {view.phase === 'loading' && <p data-test="gh-conn-loading">checking the connection…</p>}
      {view.phase === 'failed' && (
        <p className="github-settings-error" data-test="gh-conn-error">
          {view.message}
        </p>
      )}
      {view.phase === 'settled' && (
        <p data-test="gh-conn-state" data-state={view.status.state}>
          {view.status.state === 'connected'
            ? `connected to ${view.status.owner}/${view.status.repo}`
            : 'not connected'}
        </p>
      )}
      {connected && has(syncError) && (
        <p className="github-settings-error" data-test="gh-sync-error" title={syncError}>
          sync is failing — {presentSyncError(syncError)}
        </p>
      )}
      <form className="github-settings-form" onSubmit={connect} noValidate>
        <ConnectField
          label="owner"
          testId="gh-owner"
          value={owner}
          onChange={edit('owner', setOwner)}
          error={fieldErrors.owner}
        />
        <ConnectField
          label="repository"
          testId="gh-repo"
          value={repo}
          onChange={edit('repo', setRepo)}
          error={fieldErrors.repo}
        />
        <ConnectField
          label="token"
          testId="gh-token"
          type="password"
          value={token}
          onChange={edit('token', setToken)}
          error={fieldErrors.token}
        />
        <label className="github-settings-enterprise">
          <input
            data-test="gh-enterprise"
            type="checkbox"
            checked={enterprise}
            onChange={(e) => setEnterprise(e.target.checked)}
          />
          GitHub Enterprise
        </label>
        {enterprise && (
          <ConnectField
            label="API base URL"
            testId="gh-base-url"
            value={baseUrl}
            onChange={edit('baseUrl', setBaseUrl)}
            error={fieldErrors.baseUrl}
            placeholder="https://github.example.com/api/v3/"
          />
        )}
        <button type="submit" data-test="gh-connect" disabled={busy}>
          {connected ? 'reconnect' : 'connect'}
        </button>
      </form>
      {connected && (
        <button
          type="button"
          className="github-settings-disconnect"
          data-test="gh-disconnect"
          disabled={busy}
          onClick={disconnect}
        >
          disconnect
        </button>
      )}
    </dialog>
  )
}

export default GithubSettings
