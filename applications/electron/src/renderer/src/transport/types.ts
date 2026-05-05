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

// CI-state-change marker (#372). Broadcast by the backend's CI
// watcher task when the main-branch run conclusion changes. CiGlyph
// subscribes and refetches /api/ci immediately; the 15s client-side
// poll stays as the safety net. Must mirror `CI_STATE_UPDATE_MARKER`
// in applications/bpmux/src/bridge/tools_stories.rs.
export const CI_STATE_UPDATE_MARKER = '__BPMUX_CI_STATE_UPDATE__'

// Sync-status-change marker (#1164). Broadcast by the bridge's
// run_drain_loop when a drain tick mutates state (queued events
// pushed or errored). SyncStatusGlyph subscribes and refetches
// /api/sync-status immediately; the 5s client-side poll stays as
// the resilience-fallback. Must mirror `SYNC_STATUS_UPDATE_MARKER`
// in applications/bpmux/src/bridge/tools_stories.rs.
export const SYNC_STATUS_UPDATE_MARKER = '__BPMUX_SYNC_STATUS_UPDATE__'

// #402 team-roster substrate. Backend broadcasts on WS peer join /
// leave / rename; `useTeamRoster` subscribes and refetches
// `/api/roster` on receipt. Sidebar membership derives from the
// roster, not from observed message senders — so teammates stay
// visible even when they haven't spoken recently. Must mirror
// `ROSTER_UPDATE_MARKER` in applications/bpmux/src/bridge/tools_stories.rs.
export const ROSTER_UPDATE_MARKER = '__BPMUX_ROSTER_UPDATE__'
