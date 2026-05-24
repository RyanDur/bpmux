import { useEffect, useSyncExternalStore } from 'react'
import {
  ensureWsStarted,
  getWsSnapshot,
  sendChat,
  subscribeWs,
  type SendOptions
} from './wsService'
import type { MirrorState } from './state-delta'
import type { ChatMessage, ConnectionStatus } from './types'

export { RENDERER_IDENTITY } from './wsService'
export type { SendOptions } from './wsService'

export const MAX_MESSAGES = 500
export const CAP_TRIGGER_THRESHOLD = MAX_MESSAGES * 2

export interface UseWebSocketOptions {
  apiBase?: string
  authToken?: string
}

export interface UseWebSocketResult {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  sendChat: (text: string, opts?: SendOptions) => void
  mirror: MirrorState
  boardInvalidateTick: number
  syncStatusTick: number
}

export const capMessages = (messages: ChatMessage[], max: number): ChatMessage[] =>
  messages.length > max ? messages.slice(messages.length - max) : messages

export const capPerPush = (next: ChatMessage[], atBottom: boolean): ChatMessage[] => {
  if (!atBottom) return next
  if (next.length <= CAP_TRIGGER_THRESHOLD) return next
  return next.slice(next.length - MAX_MESSAGES)
}

export const useWebSocket = (
  url: string | undefined,
  opts: UseWebSocketOptions = {}
): UseWebSocketResult => {
  const { apiBase, authToken } = opts
  useEffect(() => {
    ensureWsStarted({ url, apiBase, authToken })
  }, [url, apiBase, authToken])
  const snapshot = useSyncExternalStore(subscribeWs, getWsSnapshot, getWsSnapshot)
  return {
    status: snapshot.status,
    messages: snapshot.messages,
    sessionId: snapshot.sessionId,
    sendChat,
    mirror: snapshot.mirror,
    boardInvalidateTick: snapshot.boardInvalidateTick,
    syncStatusTick: snapshot.syncStatusTick
  }
}
