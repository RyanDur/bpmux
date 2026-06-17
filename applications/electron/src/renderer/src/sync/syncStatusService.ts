import { describeHttpError, http } from '../api/http'

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

type FetchOk<T> = { ok: true; data: T }
type FetchErr = { ok: false; error: string }
type FetchResult<T> = FetchOk<T> | FetchErr

const POLL_MS = 5_000

export interface SyncStatusSnapshot {
  report: SyncStatusReport | null
  error: string | null
}

export interface SyncStatusConfig {
  apiBase: string | undefined
  authToken: string | undefined
}

const fetchSyncStatus = (
  apiBase: string,
  authToken: string,
  signal?: AbortSignal
): Promise<FetchResult<SyncStatusReport>> =>
  http.get<SyncStatusReport>({ apiBase, authToken }, '/api/sync-status', signal).mapEither(
    (data): FetchResult<SyncStatusReport> => ({ ok: true, data }),
    (err): FetchResult<SyncStatusReport> => ({
      ok: false,
      error: signal?.aborted ? '' : describeHttpError(err)
    })
  )

const postForceSync = (apiBase: string, authToken: string): Promise<FetchResult<null>> =>
  http.post({ apiBase, authToken }, '/api/sync-status/force-sync').mapEither(
    (): FetchResult<null> => ({ ok: true, data: null }),
    (err): FetchResult<null> => ({ ok: false, error: describeHttpError(err) })
  )

const postAcknowledge = (apiBase: string, authToken: string): Promise<FetchResult<null>> =>
  http.post({ apiBase, authToken }, '/api/sync-status/acknowledge').mapEither(
    (): FetchResult<null> => ({ ok: true, data: null }),
    (err): FetchResult<null> => ({ ok: false, error: describeHttpError(err) })
  )

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

const fetchNow = async (): Promise<void> => {
  const config = state.config
  if (!config?.apiBase) return
  const result = await fetchSyncStatus(
    config.apiBase,
    config.authToken ?? '',
    state.controller?.signal
  )
  if (result.ok) {
    patch({ report: result.data, error: null })
  } else if (result.error.length > 0) {
    patch({ error: result.error })
  }
}

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
  void fetchNow()
  state.pollTimer = setInterval(() => {
    void fetchNow()
  }, POLL_MS)
}

export const refreshSyncStatusNow = (): void => {
  void fetchNow()
}

const runAction = async (
  post: (apiBase: string, authToken: string) => Promise<FetchResult<null>>
): Promise<void> => {
  const config = state.config
  if (!config?.apiBase) return
  const result = await post(config.apiBase, config.authToken ?? '')
  if (!result.ok && result.error.length > 0) {
    patch({ error: result.error })
  }
  await fetchNow()
}

export const forceSync = (): Promise<void> => runAction(postForceSync)

export const acknowledge = (): Promise<void> => runAction(postAcknowledge)

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
