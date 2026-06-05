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
