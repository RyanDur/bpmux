import type { Maybe } from '@ryandur/sand'
import type { SyncStatusWire } from '@transport/types'

export type SyncState =
  | 'connected'
  | 'loading'
  | 'syncing'
  | 'offline-queued'
  | 'drained-permanent-failure'
  | 'stale'
  | 'startup-fail'

export const deriveState = (report: Maybe<SyncStatusWire>): SyncState =>
  report.map(presentState).orElse('loading')

const presentState = (report: SyncStatusWire): SyncState => {
  if (report.dropped_events.length > 0) return 'drained-permanent-failure'
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
    case 'syncing':
      return '◌'
    case 'offline-queued':
      return '⚠'
    case 'drained-permanent-failure':
      return '✗'
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
    case 'syncing':
      return `Sync state: syncing ${count} pending`
    case 'offline-queued':
      return `Sync state: offline, ${count} mutations queued`
    case 'drained-permanent-failure':
      return `Sync state: ${count} mutations dropped on permanent failure`
    case 'stale':
      return 'Sync state: stale; last successful sync exceeded the staleness threshold'
    case 'startup-fail':
      return 'Sync state: startup pull failed; board is empty until GitHub sync succeeds'
  }
}
