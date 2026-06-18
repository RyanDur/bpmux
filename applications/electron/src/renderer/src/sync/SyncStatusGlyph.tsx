import './SyncStatusGlyph.css'
import { useSyncStatus } from '../transport/session'
import type { SyncStatusWire } from '../transport/types'
import { type SyncState, deriveState, stateAriaLabel, stateGlyph } from './syncState'

const buildTooltip = (state: SyncState, report: SyncStatusWire | null): string => {
  if (!report) return 'Sync status loading…'
  const lines: string[] = []
  switch (state) {
    case 'connected':
      lines.push('Sync · connected')
      if (report.last_push_at) lines.push(`last push ${report.last_push_at}`)
      break
    case 'loading':
      lines.push('Sync · loading')
      break
    case 'syncing':
      lines.push(`Sync · syncing ${report.pending_events} pending`)
      break
    case 'offline-queued':
      lines.push(`Sync · offline, ${report.pending_events} queued`)
      if (report.last_error) lines.push(`error: ${report.last_error}`)
      break
    case 'drained-permanent-failure':
      lines.push(`Sync · ${report.dropped_events.length} dropped on permanent failure`)
      for (const d of report.dropped_events.slice(0, 3)) {
        lines.push(`#${d.story_number}: ${d.error}`)
      }
      break
    case 'recovering':
      lines.push('Sync · recovering from local-cache corruption')
      lines.push('rehydrating board state from GitHub...')
      break
    case 'stale':
      lines.push('Sync · stale')
      if (report.last_ingest_at) lines.push(`last ingest ${report.last_ingest_at}`)
      break
    case 'startup-fail':
      lines.push('Sync · startup pull failed')
      if (report.last_error) lines.push(`error: ${report.last_error}`)
      lines.push('canonical store reflects last persisted snapshot only')
      break
  }
  return lines.join('\n')
}

const SyncStatusGlyph = (): React.JSX.Element => {
  const report = useSyncStatus()
  const state = deriveState(report)
  const count =
    state === 'drained-permanent-failure'
      ? (report?.dropped_events.length ?? 0)
      : (report?.pending_events ?? 0)

  const glyph = stateGlyph(state)
  const ariaLabel = stateAriaLabel(state, count)
  const tooltip = buildTooltip(state, report)
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
