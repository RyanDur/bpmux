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
