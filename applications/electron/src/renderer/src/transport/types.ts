import type { MessageType } from './messageType'
import type { RoadmapCard, RoadmapOutcomeData } from '../board/selectRoadmap'
import type { BoardIterationData } from '../board/selectIterations'
import type { BoardColumn, Epic, Story } from '../board/types'
export interface ServerWelcome {
  type: 'welcome'
}

export interface ServerChatMessage {
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

export interface ReactionAggregateWire {
  message_key: string
  emoji: string
  count: number
  users: string[]
}

export type CiStateWire = 'unknown' | 'green' | 'red' | 'running' | 'quota_exhausted'

export interface CiSnapshotWire {
  state: CiStateWire
  run_id?: number | null
  head_sha?: string | null
  display_title?: string | null
  last_successful_poll_at?: string | null
  url?: string | null
}

interface DroppedEventWire {
  story_id: number
  error: string
  ts: string
}

export interface SyncStatusWire {
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

export interface RosterMemberWire {
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

export interface ReactionsChannelSnapshot {
  channel: string
  aggregated: ReactionAggregateWire[]
}

export interface StateSnapshotPayload {
  reactions: ReactionsChannelSnapshot[]
  ci?: CiSnapshotWire | null
  roster: RosterMemberWire[]
}

export interface CommentWire {
  id: number
  author: string
  body: string
  created_at: string
}

export interface ServerCommentsHydrated {
  type: 'comments_hydrated'
  story_id: number
  comments: CommentWire[]
}

export interface TimelineEventWire {
  event: string
  actor: string
  ts: string
  label?: string
  body?: string
}

export interface ServerTimelineHydrated {
  type: 'timeline_hydrated'
  story_id: number
  events: TimelineEventWire[]
}

export interface HistoryMessageWire {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

export interface ServerHistoryHydrated {
  type: 'history_hydrated'
  messages: HistoryMessageWire[]
}

export interface ServerHistoryPageLoaded {
  type: 'history_page_loaded'
  request_id: number
  messages: HistoryMessageWire[]
}

export interface ServerError {
  type: 'error'
  reason: string
}

export interface ServerPong {
  type: 'pong'
}

export interface ServerPtyOutput {
  type: 'pty_output'
  terminal_id: string
  data: string
}

export interface ServerPtyExit {
  type: 'pty_exit'
  terminal_id: string
}

export interface ServerPtyMode {
  type: 'pty_mode'
  terminal_id: string
  bracketed_paste: boolean
}

interface ClientIdentify {
  type: 'identify'
  name: string
  tool?: string
  owner_nonce?: string
}

interface ClientSend {
  type: 'send'
  text: string
  to?: string[]
  addressee?: string
  message_type?: string
}

interface ClientPing {
  type: 'ping'
}

interface ClientPtySubscribe {
  type: 'pty_subscribe'
  terminal_id: string
}

interface ClientPtyInput {
  type: 'pty_input'
  terminal_id: string
  data: string
}

interface ClientPtyResize {
  type: 'pty_resize'
  terminal_id: string
  cols: number
  rows: number
}

interface ClientReactionToggle {
  type: 'reaction_toggle'
  channel: string
  message_key: string
  emoji: string
  user: string
}

interface ClientCommentsHydrate {
  type: 'comments_hydrate'
  story_id: number
}

interface ClientTimelineHydrate {
  type: 'timeline_hydrate'
  story_id: number
}

interface ClientHistoryHydrate {
  type: 'history_hydrate'
  chat_limit: number
  activity_limit: number
}

interface ClientHistoryPage {
  type: 'history_page'
  request_id: number
  before: string
  limit: number
}

interface ClientSetOutcomeHorizon {
  type: 'set_outcome_horizon'
  outcome_id: number
  horizon: 'now' | 'next' | 'later'
}

interface ClientReorderOutcomeIteration {
  type: 'reorder_outcome_iteration'
  outcome_id: number
  iteration_id: number
  position: number
}

interface ClientReprioritizeOutcome {
  type: 'reprioritize_outcome'
  outcome_id: number
  position: number
}

interface ClientReorderIteration {
  type: 'reorder_iteration'
  iteration_id: number
  position: number
}

interface ClientAddStoryToIteration {
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

export interface ChatMessage {
  id: string
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  messageType?: MessageType | null
}

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected'
