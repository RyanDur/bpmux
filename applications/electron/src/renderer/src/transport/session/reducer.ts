import { applyStateDelta, applyStateSnapshot, emptyMirrorState } from '../state-delta'
import { asMessageType } from '../messageType'
import { SessionActionType, type SessionAction, type HistoryMessage } from './actions'
import type { ChatMessage, CommentWire, ConnectionStatus, TimelineEventWire } from '../types'
import type { BoardResponse, BoardStory } from '../../board/types'
import type { RoadmapOutcomeData } from '../../board/selectRoadmap'
import type { MirrorState } from '../state-delta'

export interface SessionSnapshot {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  mirror: MirrorState
  boardSnapshot: BoardResponse | null
  boardInvalidateTick: number
  reconnectTick: number
  outcomes: RoadmapOutcomeData[]
  commentsByStory: ReadonlyMap<number, CommentWire[]>
  timelineByStory: ReadonlyMap<number, TimelineEventWire[]>
}

export const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

export const emptySnapshot = (url: string | undefined): SessionSnapshot => ({
  status: url ? 'connecting' : 'disconnected',
  messages: [],
  sessionId: null,
  mirror: emptyMirrorState(),
  boardSnapshot: null,
  boardInvalidateTick: 0,
  reconnectTick: 0,
  outcomes: [],
  commentsByStory: new Map(),
  timelineByStory: new Map()
})

const mergeHistory = (state: SessionSnapshot, history: HistoryMessage[]): ChatMessage[] => {
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

export const sessionReducer = (state: SessionSnapshot, action: SessionAction): SessionSnapshot => {
  switch (action.type) {
    case SessionActionType.Connecting:
      return state.status === 'connecting' ? state : { ...state, status: 'connecting' }
    case SessionActionType.Connected:
      return { ...state, status: 'connected' }
    case SessionActionType.Reconnected:
      return { ...state, status: 'connected', reconnectTick: state.reconnectTick + 1 }
    case SessionActionType.Disconnected:
      return { ...state, status: 'disconnected' }
    case SessionActionType.SessionIdentified:
      return { ...state, sessionId: action.sessionId }
    case SessionActionType.MessageReceived: {
      const key = messageKey(action.message)
      if (state.messages.some((m) => messageKey(m) === key)) return state
      return { ...state, messages: [...state.messages, action.message] }
    }
    case SessionActionType.HistoryLoaded:
      return { ...state, messages: mergeHistory(state, action.history) }
    case SessionActionType.CommentsHydrated: {
      const commentsByStory = new Map(state.commentsByStory)
      commentsByStory.set(action.storyNumber, action.comments)
      return { ...state, commentsByStory }
    }
    case SessionActionType.TimelineHydrated: {
      const timelineByStory = new Map(state.timelineByStory)
      timelineByStory.set(action.storyNumber, action.events)
      return { ...state, timelineByStory }
    }
    case SessionActionType.MirrorSnapshotReceived:
      return { ...state, mirror: applyStateSnapshot(state.mirror, action.payload) }
    case SessionActionType.MirrorDeltaReceived: {
      const next: SessionSnapshot = {
        ...state,
        mirror: applyStateDelta(state.mirror, action.delta)
      }
      if (action.delta.domain === 'board_snapshot') {
        next.boardSnapshot = {
          stories: action.delta.stories as BoardStory[],
          fetched_at: action.ts
        }
      }
      if (action.delta.domain === 'board_invalidated') {
        next.boardInvalidateTick = state.boardInvalidateTick + 1
      }
      if (action.delta.domain === 'outcomes') {
        next.outcomes = action.delta.outcomes
      }
      return next
    }
    default:
      return state
  }
}
