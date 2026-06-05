import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import SyncStatusGlyph from './SyncStatusGlyph'
import { deriveState } from './syncState'
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

describe('#1636 deriveState — full-state coverage (null report never false-greens)', () => {
  test('null report + no error → loading (NOT connected — the false-green fix)', () => {
    expect(deriveState(null, null)).toBe('loading')
  })

  test('null report + error → unreachable (NOT connected)', () => {
    expect(deriveState(null, 'fetch failed')).toBe('unreachable')
    expect(deriveState(null, 'no backend')).toBe('unreachable')
  })

  test('a null report is NEVER connected, regardless of error', () => {
    expect(deriveState(null, null)).not.toBe('connected')
    expect(deriveState(null, 'anything')).not.toBe('connected')
  })

  test('present healthy report → connected', () => {
    expect(deriveState(report, null)).toBe('connected')
  })

  test('dropped_events → drained-permanent-failure', () => {
    expect(
      deriveState(
        {
          ...report,
          dropped_events: [{ story_number: 1, error: 'x', ts: '2026-04-22T16:00:00Z' }]
        },
        null
      )
    ).toBe('drained-permanent-failure')
  })

  test('recovering_from_corruption → recovering', () => {
    expect(deriveState({ ...report, recovering_from_corruption: true }, null)).toBe('recovering')
  })

  test('startup_fail → startup-fail', () => {
    expect(deriveState({ ...report, startup_fail: true }, null)).toBe('startup-fail')
  })

  test('pending events with a last_error → offline-queued', () => {
    expect(deriveState({ ...report, pending_events: 2, last_error: 'boom' }, null)).toBe(
      'offline-queued'
    )
  })

  test('pending events without a last_error → syncing', () => {
    expect(deriveState({ ...report, pending_events: 2 }, null)).toBe('syncing')
  })

  test('stale report → stale', () => {
    expect(deriveState({ ...report, stale: true }, null)).toBe('stale')
  })

  test('rendered with no backend: glyph is unreachable, never a false-green connected', () => {
    render(<SyncStatusGlyph apiBase={undefined} authToken={undefined} syncStatusTick={0} />)
    const glyph = screen.getByTestId('sync-status-glyph')
    expect(glyph.getAttribute('data-state')).toBe('unreachable')
    expect(glyph.getAttribute('data-state')).not.toBe('connected')
  })
})
