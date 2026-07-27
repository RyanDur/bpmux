import { applyStateDelta, emptyMirrorState } from '@transport/state-delta'
import { orderByIds, removeById, upsertById } from '@components/board/storyFolds'
import { asMessageType } from '@transport/messageType'
import { StateActionType, type StateAction, type HistoryMessage } from './actions'
import type {
  ChatMessage,
  CommentWire,
  ConnectionStatus,
  TimelineEventWire
} from '@transport/types'
import type { BoardColumn, Epic, Story } from '@components/board/types'
import type { RoadmapCard, RoadmapOutcomeData } from '@components/board/roadmap/selectRoadmap'
import type { BoardIterationData } from '@components/board/iteration/selectIterations'
import type { MirrorState } from '@transport/state-delta'

export type StateSnapshot = {
  status: ConnectionStatus
  messages: ChatMessage[]
  mirror: MirrorState
  stories: Story[]
  columns: BoardColumn[]
  epics: Epic[]
  boardInvalidateTick: number
  reconnectTick: number
  outcomes: RoadmapOutcomeData[]
  iterations: BoardIterationData[]
  roadmap: RoadmapCard[]
  commentsByStory: ReadonlyMap<number, CommentWire[]>
  timelineByStory: ReadonlyMap<number, TimelineEventWire[]>
}

const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

export const emptySnapshot = (url: string | undefined): StateSnapshot => ({
  status: url ? 'connecting' : 'disconnected',
  messages: [],
  mirror: emptyMirrorState(),
  stories: [],
  columns: [],
  epics: [],
  boardInvalidateTick: 0,
  reconnectTick: 0,
  outcomes: [],
  iterations: [],
  roadmap: [],
  commentsByStory: new Map(),
  timelineByStory: new Map()
})

const mergeHistory = (state: StateSnapshot, history: HistoryMessage[]): ChatMessage[] => {
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

export const stateReducer = (state: StateSnapshot, action: StateAction): StateSnapshot => {
  switch (action.type) {
    case StateActionType.Connecting:
      return state.status === 'connecting' ? state : { ...state, status: 'connecting' }
    case StateActionType.Connected:
      return { ...state, status: 'connected' }
    case StateActionType.Reconnected:
      return { ...state, status: 'connected', reconnectTick: state.reconnectTick + 1 }
    case StateActionType.Disconnected:
      return { ...state, status: 'disconnected' }
    case StateActionType.MessageReceived: {
      const key = messageKey(action.message)
      if (state.messages.some((m) => messageKey(m) === key)) return state
      return { ...state, messages: [...state.messages, action.message] }
    }
    case StateActionType.HistoryLoaded:
      return { ...state, messages: mergeHistory(state, action.history) }
    case StateActionType.CommentsHydrated: {
      const commentsByStory = new Map(state.commentsByStory)
      commentsByStory.set(action.storyId, action.comments)
      return { ...state, commentsByStory }
    }
    case StateActionType.TimelineHydrated: {
      const timelineByStory = new Map(state.timelineByStory)
      timelineByStory.set(action.storyId, action.events)
      return { ...state, timelineByStory }
    }
    case StateActionType.MirrorDeltaReceived: {
      const next: StateSnapshot = {
        ...state,
        mirror: applyStateDelta(state.mirror, action.delta)
      }
      if (action.delta.domain === 'stories') {
        next.stories = action.delta.stories
        if (action.delta.columns !== undefined) {
          next.columns = action.delta.columns
        }
      }
      if (action.delta.domain === 'story_upserted') {
        next.stories = upsertById(state.stories, action.delta.story)
      }
      if (action.delta.domain === 'story_removed') {
        next.stories = removeById(state.stories, action.delta.id)
      }
      if (action.delta.domain === 'story_order') {
        next.stories = orderByIds(state.stories, action.delta.ids)
      }
      if (action.delta.domain === 'epic_upserted') {
        next.epics = upsertById(state.epics, action.delta.epic)
      }
      if (action.delta.domain === 'epic_removed') {
        next.epics = removeById(state.epics, action.delta.id)
      }
      if (action.delta.domain === 'epic_order') {
        next.epics = orderByIds(state.epics, action.delta.ids)
      }
      if (action.delta.domain === 'outcome_upserted') {
        next.outcomes = upsertById(state.outcomes, action.delta.outcome)
      }
      if (action.delta.domain === 'outcome_removed') {
        next.outcomes = removeById(state.outcomes, action.delta.id)
      }
      if (action.delta.domain === 'outcome_order') {
        next.outcomes = orderByIds(state.outcomes, action.delta.ids)
      }
      if (action.delta.domain === 'iteration_upserted') {
        next.iterations = upsertById(state.iterations, action.delta.iteration)
      }
      if (action.delta.domain === 'iteration_removed') {
        next.iterations = removeById(state.iterations, action.delta.id)
      }
      if (action.delta.domain === 'iteration_order') {
        next.iterations = orderByIds(state.iterations, action.delta.ids)
      }
      if (action.delta.domain === 'roadmap_card_upserted') {
        next.roadmap = upsertById(state.roadmap, action.delta.card)
      }
      if (action.delta.domain === 'roadmap_card_removed') {
        next.roadmap = removeById(state.roadmap, action.delta.id)
      }
      if (action.delta.domain === 'roadmap_order') {
        next.roadmap = orderByIds(state.roadmap, action.delta.ids)
      }
      if (action.delta.domain === 'epics') {
        next.epics = action.delta.epics
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
