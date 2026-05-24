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

const postForceSync = async (apiBase: string, authToken: string): Promise<FetchResult<null>> => {
  const res = await fetch(`${apiBase}/api/sync-status/force-sync`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${authToken}` }
  })
  if (!res.ok) return { ok: false, error: `HTTP ${res.status}` }
  return { ok: true, data: null }
}

const postAcknowledge = async (apiBase: string, authToken: string): Promise<FetchResult<null>> => {
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
  ).catch(networkErrorToResult)
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
  const result = await post(config.apiBase, config.authToken ?? '').catch(networkErrorToResult)
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
