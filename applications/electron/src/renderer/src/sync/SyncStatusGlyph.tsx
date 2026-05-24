import { useEffect, useSyncExternalStore } from 'react'
import './SyncStatusGlyph.css'
import {
  acknowledge,
  ensureSyncStatusStarted,
  forceSync,
  getSyncStatusSnapshot,
  refreshSyncStatusNow,
  subscribeSyncStatus,
  type SyncStatusReport
} from './syncStatusService'

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

interface Props {
  apiBase?: string
  authToken?: string
  syncStatusTick?: number
}

const SyncStatusGlyph = ({ apiBase, authToken, syncStatusTick }: Props): React.JSX.Element => {
  useEffect(() => {
    ensureSyncStatusStarted({ apiBase, authToken })
  }, [apiBase, authToken])

  useEffect(() => {
    if (!syncStatusTick) return
    refreshSyncStatusNow()
  }, [syncStatusTick])

  const snapshot = useSyncExternalStore(
    subscribeSyncStatus,
    getSyncStatusSnapshot,
    getSyncStatusSnapshot
  )
  const report = snapshot.report
  const error = apiBase ? snapshot.error : 'no backend'

  const state = deriveState(report)
  const count =
    state === 'drained-permanent-failure'
      ? (report?.dropped_events.length ?? 0)
      : (report?.pending_events ?? 0)

  const handleClick = (): void => {
    if (!apiBase) return
    if (state === 'drained-permanent-failure') {
      void acknowledge()
    } else if (
      state === 'offline-queued' ||
      state === 'syncing' ||
      state === 'stale' ||
      state === 'startup-fail'
    ) {
      void forceSync()
    }
  }

  const glyph = stateGlyph(state)
  const ariaLabel = stateAriaLabel(state, count)
  const tooltip = error ? `Sync error: ${error}` : buildTooltip(state, report)
  const showCount = state !== 'connected' && count > 0

  return (
    <output aria-live="polite" className="sync-status-region">
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
        <i aria-hidden="true">{glyph}</i>
        {showCount && <small className="sync-status-glyph__count">{count}</small>}
      </button>
    </output>
  )
}

export default SyncStatusGlyph
