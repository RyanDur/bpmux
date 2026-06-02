export interface ServerWelcome {
  type: 'welcome'
  session_id: string
}

export interface ServerChatMessage {
  type: 'message'
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
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
}

export type RosterRoleWire = 'Designer' | 'Dev' | 'Pm' | 'Owner' | 'Other'

export type PresenceWire = 'Online' | 'Busy' | 'Away' | 'Offline'

export type TerminalStateWire = 'alive' | 'dead'

export interface RosterMemberWire {
  name: string
  identity: string
  role: RosterRoleWire
  presence: PresenceWire
  terminal_state?: TerminalStateWire
}

export type StateDeltaPayload =
  | { domain: 'reactions'; channel: string; aggregated: ReactionAggregateWire[] }
  | { domain: 'ci'; snapshot: CiSnapshotWire }
  | { domain: 'roster'; members: RosterMemberWire[] }
  | {
      domain: 'iteration_close'
      promoted: number[]
      deleted_marker_id: number | null
    }
  | {
      domain: 'iteration_close_blocked'
      blockers: { number: number; title: string; state: string }[]
    }

export interface ReactionsChannelSnapshot {
  channel: string
  aggregated: ReactionAggregateWire[]
}

export interface StateSnapshotPayload {
  reactions: ReactionsChannelSnapshot[]
  ci?: CiSnapshotWire | null
  roster: RosterMemberWire[]
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

export type ServerMessage =
  | ServerWelcome
  | ServerChatMessage
  | ServerError
  | ServerPong
  | ServerPtyOutput
  | ServerPtyExit

export interface ClientIdentify {
  type: 'identify'
  name: string
  tool?: string
}

export interface ClientSend {
  type: 'send'
  text: string
  to?: string[]
  addressee?: string
  message_type?: string
}

export interface ClientPing {
  type: 'ping'
}

export interface ClientPtySubscribe {
  type: 'pty_subscribe'
  terminal_id: string
}

export interface ClientPtyInput {
  type: 'pty_input'
  terminal_id: string
  data: string
}

export interface ClientPtyResize {
  type: 'pty_resize'
  terminal_id: string
  cols: number
  rows: number
}

export type ClientMessage =
  | ClientIdentify
  | ClientSend
  | ClientPing
  | ClientPtySubscribe
  | ClientPtyInput
  | ClientPtyResize

export interface ChatMessage {
  id: string
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  messageType?: string | null
}

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected'

export const BOARD_INVALIDATE_MARKER = '__BPMUX_BOARD_INVALIDATE__'

export const SYNC_STATUS_UPDATE_MARKER = '__BPMUX_SYNC_STATUS_UPDATE__'
