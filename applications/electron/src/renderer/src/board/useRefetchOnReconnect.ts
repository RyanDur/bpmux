import { useEffect, useSyncExternalStore } from 'react'
import { getSessionSnapshot, subscribeSession } from '../transport/session'

export const useRefetchOnReconnect = (refetch: () => void): void => {
  const readReconnectTick = (): number => getSessionSnapshot().reconnectTick
  const reconnectTick = useSyncExternalStore(subscribeSession, readReconnectTick, readReconnectTick)
  useEffect(() => {
    if (reconnectTick > 0) refetch()
  }, [reconnectTick, refetch])
}
