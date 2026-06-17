import { useEffect, useSyncExternalStore } from 'react'
import './SyncStatusGlyph.css'
import {
  ensureSyncStatusStarted,
  getSyncStatusSnapshot,
  refreshSyncStatusNow,
  subscribeSyncStatus,
  type SyncStatusReport
} from './syncStatusService'
import { useSyncStatusTick } from '../transport/session'
import { type SyncState, deriveState, stateAriaLabel, stateGlyph } from './syncState'

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
}

const SyncStatusGlyph = ({ apiBase, authToken }: Props): React.JSX.Element => {
  const syncStatusTick = useSyncStatusTick()
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

  const state = deriveState(report, error)
  const count =
    state === 'drained-permanent-failure'
      ? (report?.dropped_events.length ?? 0)
      : (report?.pending_events ?? 0)

  const glyph = stateGlyph(state)
  const ariaLabel = stateAriaLabel(state, count)
  const tooltip = error ? `Sync error: ${error}` : buildTooltip(state, report)
  const showCount = state !== 'connected' && count > 0

  return (
    <output
      aria-live="polite"
      className={`sync-status-region sync-status-glyph sync-status-glyph--${state}`}
      title={tooltip}
      aria-label={ariaLabel}
      data-test="sync-status-glyph"
      data-state={state}
      data-count={count}
    >
      <i aria-hidden="true">{glyph}</i>
      {showCount && <small className="sync-status-glyph__count">{count}</small>}
    </output>
  )
}

export default SyncStatusGlyph
