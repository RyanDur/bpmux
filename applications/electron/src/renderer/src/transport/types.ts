import type { MessageType } from './messageType'
import type { RoadmapCard, RoadmapOutcomeData } from '../board/roadmap/selectRoadmap'
import type { BoardIterationData } from '../board/iteration/selectIterations'
import type { BoardColumn, Epic, Story } from '../board/types'
export type ServerWelcome = {
  type: 'welcome'
}

export type ServerChatMessage = {
  type: 'message'
  from: string
  text: string
  ts: string
  sender_id?: string | null
  to?: string[] | null
  to_ids?: string[] | null
  addressee?: string | null
  addressee_ids?: string[] | null
  message_type?: string | null
  state_delta?: StateDeltaPayload | null
  state_snapshot?: StateSnapshotPayload | null
}

export type ReactionAggregateWire = {
  message_key: string
  emoji: string
  count: number
  users: string[]
}

export type CiStateWire = 'unknown' | 'green' | 'red' | 'running' | 'quota_exhausted'

export type CiSnapshotWire = {
  state: CiStateWire
  run_id?: number | null
  head_sha?: string | null
  display_title?: string | null
  last_successful_poll_at?: string | null
  url?: string | null
}

type DroppedEventWire = {
  story_id: number
  error: string
  ts: string
}

export type SyncStatusWire = {
  last_ingest_at: string | null
  last_push_at: string | null
  pending_events: number
  last_error: string | null
  dropped_events: DroppedEventWire[]
  stale: boolean
  startup_fail: boolean
}

type RosterRoleWire = 'Designer' | 'Dev' | 'Pm' | 'Owner' | 'Other'

export type PresenceWire = 'Online' | 'Busy' | 'Idle' | 'Offline' | 'Blocked' | 'Help'

export type RosterMemberWire = {
  routing_id: string
  name: string
  identity: string
  role: RosterRoleWire
  presence: PresenceWire
}

export type StateDeltaPayload =
  | { domain: 'reactions'; channel: string; aggregated: ReactionAggregateWire[] }
  | { domain: 'ci'; snapshot: CiSnapshotWire }
  | { domain: 'ci_poll'; snapshot: CiSnapshotWire }
  | { domain: 'sync_poll'; status: SyncStatusWire }
  | { domain: 'roster'; members: RosterMemberWire[] }
  | {
      domain: 'iteration_close'
      promoted: number[]
      deleted_marker_id: number | null
    }
  | {
      domain: 'iteration_close_blocked'
      blockers: { id: number; title: string; state: string }[]
    }
  | { domain: 'board_invalidated'; reason: string }
  | { domain: 'stories'; stories: Story[]; columns?: BoardColumn[] }
  | { domain: 'epics'; epics: Epic[] }
  | { domain: 'outcomes'; outcomes: RoadmapOutcomeData[] }
  | { domain: 'iterations'; iterations: BoardIterationData[] }
  | { domain: 'roadmap'; roadmap: RoadmapCard[] }

export type ReactionsChannelSnapshot = {
  channel: string
  aggregated: ReactionAggregateWire[]
}

export type StateSnapshotPayload = {
  reactions: ReactionsChannelSnapshot[]
  ci?: CiSnapshotWire | null
  roster: RosterMemberWire[]
}

export type CommentWire = {
  id: number
  author: string
  body: string
  created_at: string
}

export type ServerCommentsHydrated = {
  type: 'comments_hydrated'
  story_id: number
  comments: CommentWire[]
}

export type TimelineEventWire = {
  event: string
  actor: string
  ts: string
  label?: string
  body?: string
}

export type ServerTimelineHydrated = {
  type: 'timeline_hydrated'
  story_id: number
  events: TimelineEventWire[]
}

export type HistoryMessageWire = {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

export type ServerHistoryHydrated = {
  type: 'history_hydrated'
  messages: HistoryMessageWire[]
}

export type ServerHistoryPageLoaded = {
  type: 'history_page_loaded'
  request_id: number
  messages: HistoryMessageWire[]
}

export type ServerError = {
  type: 'error'
  reason: string
}

export type ServerPong = {
  type: 'pong'
}

export type ServerPtyOutput = {
  type: 'pty_output'
  terminal_id: string
  data: string
}

export type ServerPtyExit = {
  type: 'pty_exit'
  terminal_id: string
}

export type ServerPtyMode = {
  type: 'pty_mode'
  terminal_id: string
  bracketed_paste: boolean
}

type ClientIdentify = {
  type: 'identify'
  name: string
  tool?: string
  owner_nonce?: string
}

type ClientSend = {
  type: 'send'
  text: string
  to?: string[]
  addressee?: string
  message_type?: string
}

type ClientPing = {
  type: 'ping'
}

type ClientPtySubscribe = {
  type: 'pty_subscribe'
  terminal_id: string
}

type ClientPtyInput = {
  type: 'pty_input'
  terminal_id: string
  data: string
}

type ClientPtyResize = {
  type: 'pty_resize'
  terminal_id: string
  cols: number
  rows: number
}

type ClientReactionToggle = {
  type: 'reaction_toggle'
  channel: string
  message_key: string
  emoji: string
  user: string
}

type ClientCommentsHydrate = {
  type: 'comments_hydrate'
  story_id: number
}

type ClientTimelineHydrate = {
  type: 'timeline_hydrate'
  story_id: number
}

type ClientHistoryHydrate = {
  type: 'history_hydrate'
  chat_limit: number
  activity_limit: number
}

type ClientHistoryPage = {
  type: 'history_page'
  request_id: number
  before: string
  limit: number
}

type ClientSetOutcomeHorizon = {
  type: 'set_outcome_horizon'
  outcome_id: number
  horizon: 'now' | 'next' | 'later'
}

type ClientReorderOutcomeIteration = {
  type: 'reorder_outcome_iteration'
  outcome_id: number
  iteration_id: number
  position: number
}

type ClientReprioritizeOutcome = {
  type: 'reprioritize_outcome'
  outcome_id: number
  position: number
}

type ClientReorderIteration = {
  type: 'reorder_iteration'
  iteration_id: number
  position: number
}

type ClientAddStoryToIteration = {
  type: 'add_story_to_iteration'
  iteration_id: number
  story_id: number
  position: number
}

export type ClientMessage =
  | ClientIdentify
  | ClientSend
  | ClientPing
  | ClientPtySubscribe
  | ClientPtyInput
  | ClientPtyResize
  | ClientReactionToggle
  | ClientCommentsHydrate
  | ClientTimelineHydrate
  | ClientHistoryHydrate
  | ClientHistoryPage
  | ClientSetOutcomeHorizon
  | ClientReorderOutcomeIteration
  | ClientReprioritizeOutcome
  | ClientReorderIteration
  | ClientAddStoryToIteration

export type ChatMessage = {
  id: string
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  messageType?: MessageType | null
}

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected'
