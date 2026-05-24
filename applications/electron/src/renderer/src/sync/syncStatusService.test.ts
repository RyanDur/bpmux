import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import {
  ensureSyncStatusStarted,
  getSyncStatusSnapshot,
  refreshSyncStatusNow,
  subscribeSyncStatus,
  __resetSyncStatusForTest
} from './syncStatusService'

const fetchSpy = vi.fn()

const syncCalls = (): number =>
  fetchSpy.mock.calls.filter((c) => String(c[0]).includes('/api/sync-status')).length

const report = {
  last_ingest_at: null,
  last_push_at: null,
  pending_events: 3,
  last_error: null,
  dropped_events: [],
  recovering_from_corruption: false,
  stale: false,
  startup_fail: false
}

const okResponse = (): unknown => ({ ok: true, json: async () => report })

beforeEach(() => {
  fetchSpy.mockReset()
  fetchSpy.mockResolvedValue(okResponse())
  vi.stubGlobal('fetch', fetchSpy)
  __resetSyncStatusForTest()
})

afterEach(() => {
  __resetSyncStatusForTest()
  vi.unstubAllGlobals()
  vi.useRealTimers()
  vi.clearAllMocks()
})

describe('#1539 syncStatusService — module-level poll owner (no fetchOnceRef)', () => {
  test('ensureSyncStatusStarted fetches once immediately and publishes the report to the snapshot', async () => {
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() => expect(getSyncStatusSnapshot().report?.pending_events).toBe(3))
    expect(syncCalls()).toBe(1)
  })

  test('ensureSyncStatusStarted is idempotent — a second call with the same config does NOT start a second poll loop', async () => {
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() => expect(syncCalls()).toBe(1))
  })

  test('the poll fires again on cadence (5s interval)', async () => {
    vi.useFakeTimers()
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    await vi.advanceTimersByTimeAsync(0)
    expect(syncCalls()).toBe(1)
    await vi.advanceTimersByTimeAsync(5_000)
    expect(syncCalls()).toBe(2)
  })

  test('refreshSyncStatusNow fires an immediate extra fetch (tick path, no fetchOnceRef)', async () => {
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() => expect(syncCalls()).toBe(1))
    refreshSyncStatusNow()
    await vi.waitFor(() => expect(syncCalls()).toBe(2))
  })

  test('getSyncStatusSnapshot returns a STABLE reference until state changes', async () => {
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() => expect(getSyncStatusSnapshot().report).not.toBeNull())
    const a = getSyncStatusSnapshot()
    expect(getSyncStatusSnapshot()).toBe(a)
  })

  test('without an apiBase, no fetch is attempted', async () => {
    ensureSyncStatusStarted({ apiBase: undefined, authToken: 'tok' })
    refreshSyncStatusNow()
    await vi.waitFor(() => expect(syncCalls()).toBe(0))
  })

  test('subscribers are notified on report change and stop after unsubscribe', async () => {
    let hits = 0
    const unsub = subscribeSyncStatus(() => {
      hits += 1
    })
    ensureSyncStatusStarted({ apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() => expect(hits).toBeGreaterThan(0))
    unsub()
    const afterStart = hits
    refreshSyncStatusNow()
    await vi.waitFor(() => expect(syncCalls()).toBe(2))
    expect(hits).toBe(afterStart)
  })
})
