import { applyStateDelta, applyStateSnapshot, emptyMirrorState } from './state-delta'
import { asMessageType } from './messageType'
import { WsActionType, type WsAction, type HistoryMessage } from './wsActions'
import type { ChatMessage, ConnectionStatus } from './types'
import type { BoardResponse, BoardStory } from '../board/types'
import type { MirrorState } from './state-delta'

export interface WsSnapshot {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  mirror: MirrorState
  boardSnapshot: BoardResponse | null
  boardInvalidateTick: number
  syncStatusTick: number
  reconnectTick: number
}

export const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

export const emptySnapshot = (url: string | undefined): WsSnapshot => ({
  status: url ? 'connecting' : 'disconnected',
  messages: [],
  sessionId: null,
  mirror: emptyMirrorState(),
  boardSnapshot: null,
  boardInvalidateTick: 0,
  syncStatusTick: 0,
  reconnectTick: 0
})

const mergeHistory = (state: WsSnapshot, history: HistoryMessage[]): ChatMessage[] => {
  const seen = new Set<string>()
  const merged: ChatMessage[] = []
  for (const h of history) {
    const chat: ChatMessage = {
      id: `hist-${h.ts}-${merged.length}`,
      from: h.from,
      text: h.text,
      ts: h.ts,
      to: h.to ?? null,
      addressee: h.addressee ?? null,
      messageType: asMessageType(h.message_type)
    }
    const k = messageKey(chat)
    if (seen.has(k)) continue
    seen.add(k)
    merged.push(chat)
  }
  for (const m of state.messages) {
    const k = messageKey(m)
    if (seen.has(k)) continue
    seen.add(k)
    merged.push(m)
  }
  return merged
}

export const wsReducer = (state: WsSnapshot, action: WsAction): WsSnapshot => {
  switch (action.type) {
    case WsActionType.Connecting:
      return state.status === 'connecting' ? state : { ...state, status: 'connecting' }
    case WsActionType.Connected:
      return { ...state, status: 'connected' }
    case WsActionType.Reconnected:
      return { ...state, status: 'connected', reconnectTick: state.reconnectTick + 1 }
    case WsActionType.Disconnected:
      return { ...state, status: 'disconnected' }
    case WsActionType.SessionIdentified:
      return { ...state, sessionId: action.sessionId }
    case WsActionType.MessageReceived: {
      const key = messageKey(action.message)
      if (state.messages.some((m) => messageKey(m) === key)) return state
      return { ...state, messages: [...state.messages, action.message] }
    }
    case WsActionType.HistoryLoaded:
      return { ...state, messages: mergeHistory(state, action.history) }
    case WsActionType.MirrorSnapshotReceived:
      return { ...state, mirror: applyStateSnapshot(state.mirror, action.payload) }
    case WsActionType.MirrorDeltaReceived: {
      const next: WsSnapshot = { ...state, mirror: applyStateDelta(state.mirror, action.delta) }
      if (action.delta.domain === 'board_snapshot') {
        next.boardSnapshot = {
          stories: action.delta.stories as BoardStory[],
          fetched_at: action.ts
        }
      }
      if (action.delta.domain === 'board_invalidated') {
        next.boardInvalidateTick = state.boardInvalidateTick + 1
      }
      return next
    }
    case WsActionType.SyncStatusBumped:
      return { ...state, syncStatusTick: state.syncStatusTick + 1 }
    default:
      return state
  }
}
