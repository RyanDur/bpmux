import { useEffect, useSyncExternalStore } from 'react'
import { getStateSnapshot, subscribeState } from '../state'

export const useRefetchOnReconnect = (refetch: () => void): void => {
  const readReconnectTick = (): number => getStateSnapshot().reconnectTick
  const reconnectTick = useSyncExternalStore(subscribeState, readReconnectTick, readReconnectTick)
  useEffect(() => {
    if (reconnectTick > 0) refetch()
  }, [reconnectTick, refetch])
}
