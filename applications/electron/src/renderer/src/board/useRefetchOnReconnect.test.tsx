import { afterEach, describe, expect, test, vi } from 'vitest'
import { act, cleanup, renderHook } from '@testing-library/react'

let reconnectTick = 0
const listeners = new Set<() => void>()

vi.mock('../transport/state', () => ({
  subscribeState: (l: () => void) => {
    listeners.add(l)
    return () => listeners.delete(l)
  },
  getStateSnapshot: () => ({ reconnectTick })
}))

import { useRefetchOnReconnect } from './useRefetchOnReconnect'

const signalReconnect = (): void => {
  reconnectTick += 1
  for (const l of listeners) l()
}

afterEach(() => {
  cleanup()
  reconnectTick = 0
  listeners.clear()
})

describe('#1537 useRefetchOnReconnect — refetch on the service reconnect signal (no local ref)', () => {
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
