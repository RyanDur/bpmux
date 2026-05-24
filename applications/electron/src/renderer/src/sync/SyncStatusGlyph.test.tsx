import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import SyncStatusGlyph from './SyncStatusGlyph'
import { __resetSyncStatusForTest } from './syncStatusService'

const fetchSpy = vi.fn()

const syncCalls = (): number =>
  fetchSpy.mock.calls.filter((c) => String(c[0]).includes('/api/sync-status')).length

const report = {
  last_ingest_at: null,
  last_push_at: null,
  pending_events: 0,
  last_error: null,
  dropped_events: [],
  recovering_from_corruption: false,
  stale: false,
  startup_fail: false
}

beforeEach(() => {
  fetchSpy.mockReset()
  fetchSpy.mockResolvedValue({ ok: true, json: async () => report } as unknown as Response)
  vi.stubGlobal('fetch', fetchSpy)
  __resetSyncStatusForTest()
})

afterEach(() => {
  cleanup()
  __resetSyncStatusForTest()
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})

describe('#1456 SyncStatusGlyph — refresh survives the ingest-strip (tick replaces marker-scan)', () => {
  const props = { apiBase: 'http://127.0.0.1:9', authToken: 'tok', syncStatusTick: 0 }

  test('renders the sync-status glyph', () => {
    render(<SyncStatusGlyph {...props} />)
    expect(screen.getByTestId('sync-status-glyph')).toBeTruthy()
  })

  test('a sync-status tick increment triggers a sync-status refresh (silent-regression guard)', async () => {
    const { rerender } = render(<SyncStatusGlyph {...props} />)
    await waitFor(() => expect(syncCalls()).toBeGreaterThanOrEqual(1))
    const before = syncCalls()

    rerender(<SyncStatusGlyph {...props} syncStatusTick={1} />)

    await waitFor(() => expect(syncCalls()).toBe(before + 1))
  })
})
