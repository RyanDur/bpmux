import './SyncStatusGlyph.css'
import { useSyncStatus } from '@state'
import type { SyncStatusWire } from '@transport/types'
import { type SyncState, deriveState, stateAriaLabel, stateGlyph } from './syncState'

const presentTooltip = (state: SyncState, report: SyncStatusWire): string => {
  switch (state) {
    case 'connected':
      return [
        'Sync · connected',
        ...(report.last_push_at ? [`last push ${report.last_push_at}`] : [])
      ].join('\n')
    case 'loading':
      return 'Sync · loading'
    case 'syncing':
      return `Sync · syncing ${report.pending_events} pending`
    case 'offline-queued':
      return [
        `Sync · offline, ${report.pending_events} queued`,
        ...(report.last_error ? [`error: ${report.last_error}`] : [])
      ].join('\n')
    case 'drained-permanent-failure':
      return [
        `Sync · ${report.dropped_events.length} dropped on permanent failure`,
        ...report.dropped_events
          .slice(0, 3)
          .map((dropped) => `#${dropped.story_id}: ${dropped.error}`)
      ].join('\n')
    case 'stale':
      return [
        'Sync · stale',
        ...(report.last_ingest_at ? [`last ingest ${report.last_ingest_at}`] : [])
      ].join('\n')
    case 'startup-fail':
      return [
        'Sync · startup pull failed',
        ...(report.last_error ? [`error: ${report.last_error}`] : []),
        'board is empty until GitHub sync succeeds'
      ].join('\n')
  }
}

const SyncStatusGlyph = (): React.JSX.Element => {
  const report = useSyncStatus()
  const state = deriveState(report)
  const count = report
    .map((present) =>
      state === 'drained-permanent-failure' ? present.dropped_events.length : present.pending_events
    )
    .orElse(0)

  const glyph = stateGlyph(state)
  const ariaLabel = stateAriaLabel(state, count)
  const tooltip = report
    .map((present) => presentTooltip(state, present))
    .orElse('Sync status loading…')
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
