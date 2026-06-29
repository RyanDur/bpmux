import { applyStateDelta, applyStateSnapshot, emptyMirrorState } from '../state-delta'
import { asMessageType } from '../messageType'
import { SessionActionType, type SessionAction, type HistoryMessage } from './actions'
import type { ChatMessage, CommentWire, ConnectionStatus, TimelineEventWire } from '../types'
import type { Story, StoryId } from '../../board/types'
import type { RoadmapCard, RoadmapOutcomeData } from '../../board/selectRoadmap'
import type { BoardIterationData } from '../../board/selectIterations'
import type { MirrorState } from '../state-delta'

export interface SessionSnapshot {
  status: ConnectionStatus
  messages: ChatMessage[]
  sessionId: string | null
  mirror: MirrorState
  stories: Story[]
  boardInvalidateTick: number
  reconnectTick: number
  outcomes: RoadmapOutcomeData[]
  iterations: BoardIterationData[]
  roadmap: RoadmapCard[]
  commentsByStory: ReadonlyMap<StoryId, CommentWire[]>
  timelineByStory: ReadonlyMap<StoryId, TimelineEventWire[]>
}

export const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

export const emptySnapshot = (url: string | undefined): SessionSnapshot => ({
  status: url ? 'connecting' : 'disconnected',
  messages: [],
  sessionId: null,
  mirror: emptyMirrorState(),
  stories: [],
  boardInvalidateTick: 0,
  reconnectTick: 0,
  outcomes: [],
  iterations: [],
  roadmap: [],
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
      commentsByStory.set(action.storyId, action.comments)
      return { ...state, commentsByStory }
    }
    case SessionActionType.TimelineHydrated: {
      const timelineByStory = new Map(state.timelineByStory)
      timelineByStory.set(action.storyId, action.events)
      return { ...state, timelineByStory }
    }
    case SessionActionType.MirrorSnapshotReceived:
      return { ...state, mirror: applyStateSnapshot(state.mirror, action.payload) }
    case SessionActionType.MirrorDeltaReceived: {
      const next: SessionSnapshot = {
        ...state,
        mirror: applyStateDelta(state.mirror, action.delta)
      }
      if (action.delta.domain === 'stories') {
        next.stories = action.delta.stories
      }
      if (action.delta.domain === 'board_invalidated') {
        next.boardInvalidateTick = state.boardInvalidateTick + 1
      }
      if (action.delta.domain === 'outcomes') {
        next.outcomes = action.delta.outcomes
      }
      if (action.delta.domain === 'iterations') {
        next.iterations = action.delta.iterations
      }
      if (action.delta.domain === 'roadmap') {
        next.roadmap = action.delta.roadmap
      }
      return next
    }
    default:
      return state
  }
}
