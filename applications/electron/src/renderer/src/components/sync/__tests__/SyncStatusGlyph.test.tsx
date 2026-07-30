import { afterEach, describe, expect, test } from 'vitest'
import { act, cleanup, render, screen } from '@testing-library/react'
import SyncStatusGlyph from '../SyncStatusGlyph'
import { deriveState } from '../syncState'
import { nothing, some } from '@ryandur/sand'
import { dispatchState, resetStateForTest } from '@state/store'
import { mirrorDeltaReceived } from '@state/actions'
import type { SyncStatusWire } from '@transport/types'
const report: SyncStatusWire = {
  last_ingest_at: null,
  last_push_at: null,
  pending_events: 0,
  last_error: null,
  dropped_events: [],
  stale: false,
  startup_fail: false
}

const pushSync = (status: SyncStatusWire): void =>
  act(() => {
    dispatchState(mirrorDeltaReceived({ domain: 'sync_poll', status }, '2026-01-01T00:00:00Z'))
  })

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('SyncStatusGlyph — derives state from the WS mirror (no /api/sync-status poll)', () => {
  test('renders loading ○ until the first sync_poll heartbeat lands', () => {
    render(<SyncStatusGlyph />)
    const glyph = screen.getByTestId('sync-status-glyph')
    expect(glyph.getAttribute('data-state')).toBe('loading')
    expect(glyph.getAttribute('aria-label')).toBe('Sync state: loading — awaiting first report')
    expect(glyph.querySelector('i')?.textContent).toBe('○')
  })

  test('a healthy sync_poll heartbeat renders connected ●', () => {
    render(<SyncStatusGlyph />)
    pushSync(report)
    const glyph = screen.getByTestId('sync-status-glyph')
    expect(glyph.getAttribute('data-state')).toBe('connected')
    expect(glyph.getAttribute('aria-label')).toBe('Sync state: connected')
    expect(glyph.querySelector('i')?.textContent).toBe('●')
  })

  test('a stale heartbeat flips the glyph to stale', () => {
    render(<SyncStatusGlyph />)
    pushSync({ ...report, stale: true })
    const glyph = screen.getByTestId('sync-status-glyph')
    expect(glyph.getAttribute('data-state')).toBe('stale')
    expect(glyph.querySelector('i')?.textContent).toBe('◔')
  })

  test('pending events surface the count on the glyph', () => {
    render(<SyncStatusGlyph />)
    pushSync({ ...report, pending_events: 3 })
    const glyph = screen.getByTestId('sync-status-glyph')
    expect(glyph.getAttribute('data-state')).toBe('syncing')
    expect(glyph.getAttribute('data-count')).toBe('3')
  })
})

describe('deriveState — full-state coverage (null report never false-greens)', () => {
  test('null report → loading (NOT connected)', () => {
    expect(deriveState(nothing())).toBe('loading')
    expect(deriveState(nothing())).not.toBe('connected')
  })

  test('present healthy report → connected', () => {
    expect(deriveState(some(report))).toBe('connected')
  })

  test('dropped_events → drained-permanent-failure', () => {
    expect(
      deriveState(
        some({
          ...report,
          dropped_events: [{ story_id: 1, error: 'x', ts: '2026-04-22T16:00:00Z' }]
        })
      )
    ).toBe('drained-permanent-failure')
  })

  test('startup_fail → startup-fail', () => {
    expect(deriveState(some({ ...report, startup_fail: true }))).toBe('startup-fail')
  })

  test('pending events with a last_error → offline-queued', () => {
    expect(deriveState(some({ ...report, pending_events: 2, last_error: 'boom' }))).toBe(
      'offline-queued'
    )
  })

  test('pending events without a last_error → syncing', () => {
    expect(deriveState(some({ ...report, pending_events: 2 }))).toBe('syncing')
  })

  test('stale report → stale', () => {
    expect(deriveState(some({ ...report, stale: true }))).toBe('stale')
  })
})
