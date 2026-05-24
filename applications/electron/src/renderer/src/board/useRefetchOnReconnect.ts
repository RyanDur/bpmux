import { useEffect, useSyncExternalStore } from 'react'
import { getWsSnapshot, subscribeWs } from '../transport/wsService'

export const useRefetchOnReconnect = (refetch: () => void): void => {
  const readReconnectTick = (): number => getWsSnapshot().reconnectTick
  const reconnectTick = useSyncExternalStore(subscribeWs, readReconnectTick, readReconnectTick)
  useEffect(() => {
    if (reconnectTick > 0) refetch()
  }, [reconnectTick, refetch])
}
