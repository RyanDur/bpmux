import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { dispatchSession, resetSessionForTest } from './store'
import { connected, mirrorDeltaReceived, sessionIdentified } from './actions'
import { useConnectionStatus, useRoster, useSessionId, useSyncStatus } from './hooks'

afterEach(() => resetSessionForTest())

describe('session selector hooks', () => {
  it('useConnectionStatus reflects connection events', () => {
    const { result } = renderHook(() => useConnectionStatus())
    expect(result.current).toBe('disconnected')
    act(() => dispatchSession(connected()))
    expect(result.current).toBe('connected')
  })

  it('useSessionId reflects the identify event', () => {
    const { result } = renderHook(() => useSessionId())
    expect(result.current).toBeNull()
    act(() => dispatchSession(sessionIdentified('sess-1')))
    expect(result.current).toBe('sess-1')
  })

  it('useRoster reflects a roster mirror-delta', () => {
    const { result } = renderHook(() => useRoster())
    expect(result.current).toEqual([])
    act(() =>
      dispatchSession(
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
    expect(result.current).toBeNull()
    act(() =>
      dispatchSession(
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
              startup_fail: false,
              conflicts_pending: 0
            }
          },
          '2026-01-01T00:00:00Z'
        )
      )
    )
    expect(result.current?.pending_events).toBe(2)
  })
})
