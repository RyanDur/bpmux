import { type ApiContext, type HttpError, describeHttpError, http } from '../api/http'
import { type AsyncResult, asyncSuccess } from '../lib/asyncResult'

export interface DroppedEvent {
  story_number: number
  error: string
  ts: string
}

export interface SyncStatusReport {
  last_ingest_at: string | null
  last_push_at: string | null
  pending_events: number
  last_error: string | null
  dropped_events: DroppedEvent[]
  recovering_from_corruption: boolean
  stale: boolean
  startup_fail: boolean
}

const POLL_MS = 5_000

export interface SyncStatusSnapshot {
  report: SyncStatusReport | null
  error: string | null
}

export interface SyncStatusConfig {
  apiBase: string | undefined
  authToken: string | undefined
}

interface ServiceState {
  config: SyncStatusConfig | null
  snapshot: SyncStatusSnapshot
  pollTimer: ReturnType<typeof setInterval> | null
  controller: AbortController | null
  listeners: Set<() => void>
}

const freshState = (): ServiceState => ({
  config: null,
  snapshot: { report: null, error: null },
  pollTimer: null,
  controller: null,
  listeners: new Set()
})

let state: ServiceState = freshState()

const patch = (changes: Partial<SyncStatusSnapshot>): void => {
  state.snapshot = { ...state.snapshot, ...changes }
  for (const listener of state.listeners) listener()
}

const fetchNow = (ctx: ApiContext): AsyncResult<SyncStatusReport, HttpError> =>
  http
    .get<SyncStatusReport>(ctx, '/api/sync-status', state.controller?.signal)
    .onSuccess((report) => patch({ report, error: null }))
    .onFailure((err) => {
      if (!state.controller?.signal.aborted) patch({ error: describeHttpError(err) })
    })

const sameConfig = (a: SyncStatusConfig | null, b: SyncStatusConfig): boolean =>
  a !== null && a.apiBase === b.apiBase && a.authToken === b.authToken

const teardown = (): void => {
  if (state.pollTimer) {
    clearInterval(state.pollTimer)
    state.pollTimer = null
  }
  state.controller?.abort()
  state.controller = null
}

export const ensureSyncStatusStarted = (config: SyncStatusConfig): void => {
  if (sameConfig(state.config, config)) return
  teardown()
  state.config = config
  if (!config.apiBase) return
  state.controller = new AbortController()
  const ctx: ApiContext = { apiBase: config.apiBase, authToken: config.authToken }
  fetchNow(ctx)
  state.pollTimer = setInterval(() => fetchNow(ctx), POLL_MS)
}

export const refreshSyncStatusNow = (): void => {
  const config = state.config
  if (!config?.apiBase) return
  fetchNow({ apiBase: config.apiBase, authToken: config.authToken })
}

const runAction = (endpoint: string): void => {
  const config = state.config
  if (!config?.apiBase) return
  const ctx: ApiContext = { apiBase: config.apiBase, authToken: config.authToken }
  http
    .post(ctx, endpoint)
    .or((err) => {
      patch({ error: describeHttpError(err) })
      return asyncSuccess<void, HttpError>(undefined)
    })
    .andThen(() => fetchNow(ctx))
}

export const forceSync = (): void => runAction('/api/sync-status/force-sync')

export const acknowledge = (): void => runAction('/api/sync-status/acknowledge')

export const subscribeSyncStatus = (listener: () => void): (() => void) => {
  state.listeners.add(listener)
  return () => {
    state.listeners.delete(listener)
  }
}

export const getSyncStatusSnapshot = (): SyncStatusSnapshot => state.snapshot

export const __resetSyncStatusForTest = (): void => {
  teardown()
  state = freshState()
}
