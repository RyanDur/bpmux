import type { ChatMessage, StateDeltaPayload, StateSnapshotPayload } from '../types'

export interface SendOptions {
  addressee?: string | null
  to?: string[] | null
}

export interface SessionConfig {
  url: string | undefined
  apiBase?: string
  authToken?: string
}

export interface HistoryMessage {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

export const SessionActionType = {
  Connecting: 'session/connecting',
  Connected: 'session/connected',
  Reconnected: 'session/reconnected',
  Disconnected: 'session/disconnected',
  SessionIdentified: 'session/identified',
  MessageReceived: 'session/messageReceived',
  HistoryLoaded: 'session/historyLoaded',
  MirrorSnapshotReceived: 'session/mirrorSnapshotReceived',
  MirrorDeltaReceived: 'session/mirrorDeltaReceived',
  PtyOutputReceived: 'session/ptyOutputReceived',
  PtyExitReceived: 'session/ptyExitReceived',
  StartRequested: 'session/startRequested',
  ChatSendRequested: 'session/chatSendRequested',
  ReactionToggleRequested: 'session/reactionToggleRequested',
  PtySubscribeRequested: 'session/ptySubscribeRequested',
  PtyInputRequested: 'session/ptyInputRequested',
  PtyResizeRequested: 'session/ptyResizeRequested',
  FrameReceived: 'session/frameReceived',
  Reset: 'session/reset'
} as const

export type SessionAction =
  | { type: typeof SessionActionType.Connecting }
  | { type: typeof SessionActionType.Connected }
  | { type: typeof SessionActionType.Reconnected }
  | { type: typeof SessionActionType.Disconnected }
  | { type: typeof SessionActionType.SessionIdentified; sessionId: string }
  | { type: typeof SessionActionType.MessageReceived; message: ChatMessage }
  | { type: typeof SessionActionType.HistoryLoaded; history: HistoryMessage[] }
  | { type: typeof SessionActionType.MirrorSnapshotReceived; payload: StateSnapshotPayload }
  | { type: typeof SessionActionType.MirrorDeltaReceived; delta: StateDeltaPayload; ts: string }
  | { type: typeof SessionActionType.PtyOutputReceived; terminalId: string; data: string }
  | { type: typeof SessionActionType.PtyExitReceived; terminalId: string }
  | { type: typeof SessionActionType.StartRequested; config: SessionConfig }
  | { type: typeof SessionActionType.ChatSendRequested; text: string; opts?: SendOptions }
  | {
      type: typeof SessionActionType.ReactionToggleRequested
      channel: string
      messageKey: string
      emoji: string
      user: string
    }
  | { type: typeof SessionActionType.PtySubscribeRequested; terminalId: string }
  | { type: typeof SessionActionType.PtyInputRequested; terminalId: string; keystroke: string }
  | {
      type: typeof SessionActionType.PtyResizeRequested
      terminalId: string
      cols: number
      rows: number
    }
  | { type: typeof SessionActionType.FrameReceived; raw: string }
  | { type: typeof SessionActionType.Reset }

export const connecting = (): SessionAction => ({ type: SessionActionType.Connecting })
export const connected = (): SessionAction => ({ type: SessionActionType.Connected })
export const reconnected = (): SessionAction => ({ type: SessionActionType.Reconnected })
export const disconnected = (): SessionAction => ({ type: SessionActionType.Disconnected })

export const sessionIdentified = (sessionId: string): SessionAction => ({
  type: SessionActionType.SessionIdentified,
  sessionId
})

export const messageReceived = (message: ChatMessage): SessionAction => ({
  type: SessionActionType.MessageReceived,
  message
})

export const historyLoaded = (history: HistoryMessage[]): SessionAction => ({
  type: SessionActionType.HistoryLoaded,
  history
})

export const mirrorSnapshotReceived = (payload: StateSnapshotPayload): SessionAction => ({
  type: SessionActionType.MirrorSnapshotReceived,
  payload
})

export const mirrorDeltaReceived = (delta: StateDeltaPayload, ts: string): SessionAction => ({
  type: SessionActionType.MirrorDeltaReceived,
  delta,
  ts
})

export const ptyOutputReceived = (terminalId: string, data: string): SessionAction => ({
  type: SessionActionType.PtyOutputReceived,
  terminalId,
  data
})

export const ptyExitReceived = (terminalId: string): SessionAction => ({
  type: SessionActionType.PtyExitReceived,
  terminalId
})

export const startRequested = (config: SessionConfig): SessionAction => ({
  type: SessionActionType.StartRequested,
  config
})

export const chatSendRequested = (text: string, opts?: SendOptions): SessionAction => ({
  type: SessionActionType.ChatSendRequested,
  text,
  opts
})

export const reactionToggleRequested = (
  channel: string,
  messageKey: string,
  emoji: string,
  user: string
): SessionAction => ({
  type: SessionActionType.ReactionToggleRequested,
  channel,
  messageKey,
  emoji,
  user
})

export const ptySubscribeRequested = (terminalId: string): SessionAction => ({
  type: SessionActionType.PtySubscribeRequested,
  terminalId
})

export const ptyInputRequested = (terminalId: string, keystroke: string): SessionAction => ({
  type: SessionActionType.PtyInputRequested,
  terminalId,
  keystroke
})

export const ptyResizeRequested = (
  terminalId: string,
  cols: number,
  rows: number
): SessionAction => ({
  type: SessionActionType.PtyResizeRequested,
  terminalId,
  cols,
  rows
})

export const frameReceived = (raw: string): SessionAction => ({
  type: SessionActionType.FrameReceived,
  raw
})

export const reset = (): SessionAction => ({ type: SessionActionType.Reset })
