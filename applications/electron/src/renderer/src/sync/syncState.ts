import type { SyncStatusReport } from './syncStatusService'

export type SyncState =
  | 'connected'
  | 'loading'
  | 'unreachable'
  | 'syncing'
  | 'offline-queued'
  | 'drained-permanent-failure'
  | 'recovering'
  | 'stale'
  | 'startup-fail'

export const deriveState = (report: SyncStatusReport | null, error: string | null): SyncState => {
  if (!report) return error ? 'unreachable' : 'loading'
  if (report.dropped_events.length > 0) return 'drained-permanent-failure'
  if (report.recovering_from_corruption) return 'recovering'
  if (report.startup_fail) return 'startup-fail'
  if (report.pending_events > 0 && report.last_error !== null) return 'offline-queued'
  if (report.pending_events > 0) return 'syncing'
  if (report.stale) return 'stale'
  return 'connected'
}

export const stateGlyph = (state: SyncState): string => {
  switch (state) {
    case 'connected':
      return '●'
    case 'loading':
      return '○'
    case 'unreachable':
      return '?'
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

export const stateAriaLabel = (state: SyncState, count: number): string => {
  switch (state) {
    case 'connected':
      return 'Sync state: connected'
    case 'loading':
      return 'Sync state: loading — awaiting first report'
    case 'unreachable':
      return 'Sync state: unknown — sync status unreachable'
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
