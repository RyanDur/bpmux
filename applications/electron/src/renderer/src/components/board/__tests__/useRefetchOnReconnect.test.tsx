import { afterEach, describe, expect, test, vi } from 'vitest'
import { act, cleanup, renderHook } from '@testing-library/react'

import { useRefetchOnReconnect } from '../useRefetchOnReconnect'
import { dispatchState, resetStateForTest } from '@state/store'
import { reconnected } from '@state/actions'

const signalReconnect = (): void => {
  dispatchState(reconnected())
}

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('useRefetchOnReconnect — refetch on the service reconnect signal (no local ref)', () => {
  test('does NOT refetch on mount when no reconnect has occurred (tick 0)', () => {
    const refetch = vi.fn()
    renderHook(() => useRefetchOnReconnect(refetch))
    expect(refetch).not.toHaveBeenCalled()
  })

  test('refetches when the service signals a reconnect (disconnect→connect tick increment)', () => {
    const refetch = vi.fn()
    renderHook(() => useRefetchOnReconnect(refetch))
    act(() => signalReconnect())
    expect(refetch).toHaveBeenCalledTimes(1)
  })

  test('refetches once per reconnect, not on every render (stable-refetch contract)', () => {
    const refetch = vi.fn()
    const { rerender } = renderHook(() => useRefetchOnReconnect(refetch))
    act(() => signalReconnect())
    rerender()
    rerender()
    expect(refetch).toHaveBeenCalledTimes(1)
    act(() => signalReconnect())
    expect(refetch).toHaveBeenCalledTimes(2)
  })
})
