// Wire-protocol types mirroring
// `applications/bpmux/src/transport/protocol.rs`. Keep in sync with the
// Rust `ClientMessage` / `ServerMessage` enums — both serialize with
// `#[serde(tag = "type")]`, so the discriminator field is `type` and
// values are lowercased variant names.

// ─── server → client ─────────────────────────────────────────────────

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

export interface PinnedMessageWire {
  message_key: string
  pinner: string
  ts: string
  text: string
  from: string
  pinned_at: string
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

export interface RosterMemberWire {
  name: string
  identity: string
  role: RosterRoleWire
  presence: PresenceWire
}

export type StateDeltaPayload =
  | { domain: 'reactions'; channel: string; aggregated: ReactionAggregateWire[] }
  | { domain: 'pins'; channel: string; pins: PinnedMessageWire[] }
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
  | { domain: 'icebox_migration'; entries: number }

export interface ReactionsChannelSnapshot {
  channel: string
  aggregated: ReactionAggregateWire[]
}

export interface PinsChannelSnapshot {
  channel: string
  pins: PinnedMessageWire[]
}

export interface StateSnapshotPayload {
  reactions: ReactionsChannelSnapshot[]
  pins: PinsChannelSnapshot[]
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

export type ServerMessage = ServerWelcome | ServerChatMessage | ServerError | ServerPong

// ─── client → server ─────────────────────────────────────────────────

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

export type ClientMessage = ClientIdentify | ClientSend | ClientPing

// ─── renderer-local view model ───────────────────────────────────────

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

// Board-invalidate marker — broadcast by the backend when board
// state changes (accept/reject/reorder/comment). Renderer components
// that cache data (board list, drill-in comments) subscribe by
// watching the message stream for this marker text and refetch on
// match. Must exactly mirror `BOARD_INVALIDATE_MARKER` in
// applications/bpmux/src/bridge/tools_stories.rs.
export const BOARD_INVALIDATE_MARKER = '__BPMUX_BOARD_INVALIDATE__'

// Sync-status-change marker (#1164). Broadcast by the bridge's
// run_drain_loop when a drain tick mutates state (queued events
// pushed or errored). SyncStatusGlyph subscribes and refetches
// /api/sync-status immediately; the 5s client-side poll stays as
// the resilience-fallback. Must mirror `SYNC_STATUS_UPDATE_MARKER`
// in applications/bpmux/src/bridge/tools_stories.rs.
export const SYNC_STATUS_UPDATE_MARKER = '__BPMUX_SYNC_STATUS_UPDATE__'
