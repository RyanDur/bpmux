import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { dispatchSession, resetSessionForTest } from './store'
import { connected, mirrorDeltaReceived, sessionIdentified, syncStatusBumped } from './actions'
import { useConnectionStatus, useRoster, useSessionId, useSyncStatusTick } from './hooks'

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

  it('useSyncStatusTick increments on a sync-status bump', () => {
    const { result } = renderHook(() => useSyncStatusTick())
    expect(result.current).toBe(0)
    act(() => dispatchSession(syncStatusBumped()))
    expect(result.current).toBe(1)
  })
})
