import { useEffect, useRef } from 'react'
import type { ConnectionStatus } from '../transport/types'

export interface ReconnectDecision {
  wasDisconnected: boolean
  shouldRefetch: boolean
}

export function decideRefetchOnStatus(
  status: ConnectionStatus,
  wasDisconnected: boolean
): ReconnectDecision {
  if (status === 'disconnected') {
    return { wasDisconnected: true, shouldRefetch: false }
  }
  if (status === 'connected' && wasDisconnected) {
    return { wasDisconnected: false, shouldRefetch: true }
  }
  return { wasDisconnected, shouldRefetch: false }
}

export function useRefetchOnReconnect(
  status: ConnectionStatus,
  refetch: () => void
): void {
  const wasDisconnectedRef = useRef<boolean>(false)
  useEffect(() => {
    const decision = decideRefetchOnStatus(status, wasDisconnectedRef.current)
    wasDisconnectedRef.current = decision.wasDisconnected
    if (decision.shouldRefetch) {
      refetch()
    }
  }, [status, refetch])
}
