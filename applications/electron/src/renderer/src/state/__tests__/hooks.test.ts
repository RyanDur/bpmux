import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { dispatchState, resetStateForTest } from '../store'
import { connected, mirrorDeltaReceived } from '../actions'
import { useConnectionStatus, useRoster, useSyncStatus } from '../hooks'

afterEach(() => resetStateForTest())

describe('state selector hooks', () => {
  it('useConnectionStatus reflects connection events', () => {
    const { result } = renderHook(() => useConnectionStatus())
    expect(result.current).toBe('disconnected')
    act(() => dispatchState(connected()))
    expect(result.current).toBe('connected')
  })

  it('useRoster reflects a roster mirror-delta', () => {
    const { result } = renderHook(() => useRoster())
    expect(result.current).toEqual([])
    act(() =>
      dispatchState(
        mirrorDeltaReceived(
          {
            domain: 'roster',
            members: [
              { routing_id: 'r1', name: 'pm', identity: 'pm', role: 'Pm', presence: 'Idle' }
            ]
          },
          '2026-01-01T00:00:00Z'
        )
      )
    )
    expect(result.current.map((m) => m.role)).toEqual(['Pm'])
  })

  it('useSyncStatus reflects a sync_poll mirror-delta', () => {
    const { result } = renderHook(() => useSyncStatus())
    expect(result.current.isNothing).toBe(true)
    act(() =>
      dispatchState(
        mirrorDeltaReceived(
          {
            domain: 'sync_poll',
            status: {
              last_ingest_at: '2026-01-01T00:00:00Z',
              last_push_at: null,
              pending_events: 2,
              last_error: null,
              dropped_events: [],
              stale: false,
              startup_fail: false
            }
          },
          '2026-01-01T00:00:00Z'
        )
      )
    )
    expect(result.current.map((status) => status.pending_events).orNull()).toBe(2)
  })
})
