import { useCallback, useEffect, useState } from 'react'
import './SyncStatusGlyph.css'

interface DroppedEvent {
  story_number: number
  error: string
  ts: string
}

interface SyncStatusReport {
  last_ingest_at: string | null
  last_push_at: string | null
  pending_events: number
  last_error: string | null
  dropped_events: DroppedEvent[]
  recovering_from_corruption: boolean
  stale: boolean
  startup_fail: boolean
}

type FetchOk<T> = { ok: true; data: T }
type FetchErr = { ok: false; error: string }
type FetchResult<T> = FetchOk<T> | FetchErr

const POLL_MS = 5_000

type SyncState =
  | 'connected'
  | 'syncing'
  | 'offline-queued'
  | 'drained-permanent-failure'
  | 'recovering'
  | 'stale'
  | 'startup-fail'

const deriveState = (report: SyncStatusReport | null): SyncState => {
  if (!report) return 'connected'
  if (report.dropped_events.length > 0) return 'drained-permanent-failure'
  if (report.recovering_from_corruption) return 'recovering'
  if (report.startup_fail) return 'startup-fail'
  if (report.pending_events > 0 && report.last_error !== null) return 'offline-queued'
  if (report.pending_events > 0) return 'syncing'
  if (report.stale) return 'stale'
  return 'connected'
}

const stateGlyph = (state: SyncState): string => {
  switch (state) {
    case 'connected':
      return '●'
    case 'syncing':
      return '◌'
    case 'offline-queued':
      return '⚠'
    case 'drained-permanent-failure':
      return '✗'
    case 'recovering':
      return '↻'
    case 'stale':
      return '◔'
    case 'startup-fail':
      return '⊘'
  }
}

const stateAriaLabel = (state: SyncState, count: number): string => {
  switch (state) {
    case 'connected':
      return 'Sync state: connected'
    case 'syncing':
      return `Sync state: syncing ${count} pending`
    case 'offline-queued':
      return `Sync state: offline, ${count} mutations queued`
    case 'drained-permanent-failure':
      return `Sync state: ${count} mutations dropped on permanent failure`
    case 'recovering':
      return 'Sync state: recovering from local-cache corruption; rehydrating from GitHub'
    case 'stale':
      return 'Sync state: stale; last successful sync exceeded the staleness threshold'
    case 'startup-fail':
      return 'Sync state: startup pull failed; canonical store reflects last persisted snapshot only'
  }
}

const buildTooltip = (state: SyncState, report: SyncStatusReport | null): string => {
  if (!report) return 'Sync status loading…'
  const lines: string[] = []
  switch (state) {
    case 'connected':
      lines.push('Sync · connected')
      if (report.last_push_at) lines.push(`last push ${report.last_push_at}`)
      break
    case 'syncing':
      lines.push(`Sync · syncing ${report.pending_events} pending`)
      break
    case 'offline-queued':
      lines.push(`Sync · offline, ${report.pending_events} queued`)
      if (report.last_error) lines.push(`error: ${report.last_error}`)
      lines.push('(click to force-sync now)')
      break
    case 'drained-permanent-failure':
      lines.push(`Sync · ${report.dropped_events.length} dropped on permanent failure`)
      for (const d of report.dropped_events.slice(0, 3)) {
        lines.push(`#${d.story_number}: ${d.error}`)
      }
      lines.push('(click to acknowledge)')
      break
    case 'recovering':
      lines.push('Sync · recovering from local-cache corruption')
      lines.push('rehydrating board state from GitHub...')
      break
    case 'stale':
      lines.push('Sync · stale')
      if (report.last_ingest_at) lines.push(`last ingest ${report.last_ingest_at}`)
      lines.push('(click to force-sync now)')
      break
    case 'startup-fail':
      lines.push('Sync · startup pull failed')
      if (report.last_error) lines.push(`error: ${report.last_error}`)
      lines.push('canonical store reflects last persisted snapshot only')
      lines.push('(click to retry; check token / network if persists)')
      break
  }
  return lines.join('\n')
}

const fetchSyncStatus = async (
  apiBase: string,
  authToken: string,
  signal?: AbortSignal
): Promise<FetchResult<SyncStatusReport>> => {
  const res = await fetch(`${apiBase}/api/sync-status`, {
    headers: { Authorization: `Bearer ${authToken}` },
    signal
  })
  if (!res.ok) return { ok: false, error: `HTTP ${res.status}` }
  const data = (await res.json()) as SyncStatusReport
  return { ok: true, data }
}

const postForceSync = async (
  apiBase: string,
  authToken: string
): Promise<FetchResult<null>> => {
  const res = await fetch(`${apiBase}/api/sync-status/force-sync`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${authToken}` }
  })
  if (!res.ok) return { ok: false, error: `HTTP ${res.status}` }
  return { ok: true, data: null }
}

const postAcknowledge = async (
  apiBase: string,
  authToken: string
): Promise<FetchResult<null>> => {
  const res = await fetch(`${apiBase}/api/sync-status/acknowledge`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${authToken}` }
  })
  if (!res.ok) return { ok: false, error: `HTTP ${res.status}` }
  return { ok: true, data: null }
}

const networkErrorToResult = (err: unknown): FetchErr => ({
  ok: false,
  error: (err as Error)?.name === 'AbortError' ? '' : String(err)
})

interface Props {
  apiBase?: string
  authToken?: string
}

const SyncStatusGlyph = ({ apiBase, authToken }: Props): React.JSX.Element => {
  const [report, setReport] = useState<SyncStatusReport | null>(null)
  const [error, setError] = useState<string | null>(null)

  const fetchOnce = useCallback(
    async (signal?: AbortSignal): Promise<void> => {
      if (!apiBase) return
      const result = await fetchSyncStatus(apiBase, authToken ?? '', signal).catch(
        networkErrorToResult
      )
      if (result.ok) {
        setReport(result.data)
        setError(null)
      } else if (result.error.length > 0) {
        setError(result.error)
      }
    },
    [apiBase, authToken]
  )

  useEffect(() => {
    if (!apiBase) {
      setError('no backend')
      return
    }
    const controller = new AbortController()
    void fetchOnce(controller.signal)
    const id = setInterval(() => {
      void fetchOnce(controller.signal)
    }, POLL_MS)
    return () => {
      controller.abort()
      clearInterval(id)
    }
  }, [apiBase, fetchOnce])

  const state = deriveState(report)
  const count =
    state === 'drained-permanent-failure'
      ? (report?.dropped_events.length ?? 0)
      : (report?.pending_events ?? 0)

  const handleClick = useCallback(async () => {
    if (!apiBase) return
    const token = authToken ?? ''
    let result: FetchResult<null> | null = null
    if (state === 'drained-permanent-failure') {
      result = await postAcknowledge(apiBase, token).catch(networkErrorToResult)
    } else if (
      state === 'offline-queued' ||
      state === 'syncing' ||
      state === 'stale' ||
      state === 'startup-fail'
    ) {
      result = await postForceSync(apiBase, token).catch(networkErrorToResult)
    }
    if (result && !result.ok && result.error.length > 0) {
      setError(result.error)
    }
    await fetchOnce()
  }, [apiBase, authToken, fetchOnce, state])

  const glyph = stateGlyph(state)
  const ariaLabel = stateAriaLabel(state, count)
  const tooltip = error ? `Sync error: ${error}` : buildTooltip(state, report)
  const showCount = state !== 'connected' && count > 0

  return (
    <aside role="status" aria-live="polite" className="sync-status-region">
      <button
        type="button"
        className={`sync-status-glyph sync-status-glyph--${state}`}
        onClick={handleClick}
        title={tooltip}
        aria-label={ariaLabel}
        data-test="sync-status-glyph"
        data-state={state}
        data-count={count}
      >
        <span aria-hidden="true">{glyph}</span>
        {showCount && <small className="sync-status-glyph__count">{count}</small>}
      </button>
    </aside>
  )
}

export default SyncStatusGlyph
