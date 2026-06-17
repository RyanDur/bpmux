import type { ChatMessage, StateDeltaPayload, StateSnapshotPayload } from './types'

export interface SendOptions {
  addressee?: string | null
  to?: string[] | null
}

export interface WsServiceConfig {
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

export const WsActionType = {
  Connecting: 'ws/connecting',
  Connected: 'ws/connected',
  Reconnected: 'ws/reconnected',
  Disconnected: 'ws/disconnected',
  SessionIdentified: 'ws/sessionIdentified',
  MessageReceived: 'ws/messageReceived',
  HistoryLoaded: 'ws/historyLoaded',
  MirrorSnapshotReceived: 'ws/mirrorSnapshotReceived',
  MirrorDeltaReceived: 'ws/mirrorDeltaReceived',
  SyncStatusBumped: 'ws/syncStatusBumped',
  PtyOutputReceived: 'ws/ptyOutputReceived',
  PtyExitReceived: 'ws/ptyExitReceived',
  StartRequested: 'ws/startRequested',
  ChatSendRequested: 'ws/chatSendRequested',
  PtySubscribeRequested: 'ws/ptySubscribeRequested',
  PtyInputRequested: 'ws/ptyInputRequested',
  PtyResizeRequested: 'ws/ptyResizeRequested'
} as const

export type WsAction =
  | { type: typeof WsActionType.Connecting }
  | { type: typeof WsActionType.Connected }
  | { type: typeof WsActionType.Reconnected }
  | { type: typeof WsActionType.Disconnected }
  | { type: typeof WsActionType.SessionIdentified; sessionId: string }
  | { type: typeof WsActionType.MessageReceived; message: ChatMessage }
  | { type: typeof WsActionType.HistoryLoaded; history: HistoryMessage[] }
  | { type: typeof WsActionType.MirrorSnapshotReceived; payload: StateSnapshotPayload }
  | { type: typeof WsActionType.MirrorDeltaReceived; delta: StateDeltaPayload; ts: string }
  | { type: typeof WsActionType.SyncStatusBumped }
  | { type: typeof WsActionType.PtyOutputReceived; terminalId: string; data: string }
  | { type: typeof WsActionType.PtyExitReceived; terminalId: string }
  | { type: typeof WsActionType.StartRequested; config: WsServiceConfig }
  | { type: typeof WsActionType.ChatSendRequested; text: string; opts?: SendOptions }
  | { type: typeof WsActionType.PtySubscribeRequested; terminalId: string }
  | { type: typeof WsActionType.PtyInputRequested; terminalId: string; keystroke: string }
  | { type: typeof WsActionType.PtyResizeRequested; terminalId: string; cols: number; rows: number }

export const connecting = (): WsAction => ({ type: WsActionType.Connecting })
export const connected = (): WsAction => ({ type: WsActionType.Connected })
export const reconnected = (): WsAction => ({ type: WsActionType.Reconnected })
export const disconnected = (): WsAction => ({ type: WsActionType.Disconnected })

export const sessionIdentified = (sessionId: string): WsAction => ({
  type: WsActionType.SessionIdentified,
  sessionId
})

export const messageReceived = (message: ChatMessage): WsAction => ({
  type: WsActionType.MessageReceived,
  message
})

export const historyLoaded = (history: HistoryMessage[]): WsAction => ({
  type: WsActionType.HistoryLoaded,
  history
})

export const mirrorSnapshotReceived = (payload: StateSnapshotPayload): WsAction => ({
  type: WsActionType.MirrorSnapshotReceived,
  payload
})

export const mirrorDeltaReceived = (delta: StateDeltaPayload, ts: string): WsAction => ({
  type: WsActionType.MirrorDeltaReceived,
  delta,
  ts
})

export const syncStatusBumped = (): WsAction => ({ type: WsActionType.SyncStatusBumped })

export const ptyOutputReceived = (terminalId: string, data: string): WsAction => ({
  type: WsActionType.PtyOutputReceived,
  terminalId,
  data
})

export const ptyExitReceived = (terminalId: string): WsAction => ({
  type: WsActionType.PtyExitReceived,
  terminalId
})

export const startRequested = (config: WsServiceConfig): WsAction => ({
  type: WsActionType.StartRequested,
  config
})

export const chatSendRequested = (text: string, opts?: SendOptions): WsAction => ({
  type: WsActionType.ChatSendRequested,
  text,
  opts
})

export const ptySubscribeRequested = (terminalId: string): WsAction => ({
  type: WsActionType.PtySubscribeRequested,
  terminalId
})

export const ptyInputRequested = (terminalId: string, keystroke: string): WsAction => ({
  type: WsActionType.PtyInputRequested,
  terminalId,
  keystroke
})

export const ptyResizeRequested = (terminalId: string, cols: number, rows: number): WsAction => ({
  type: WsActionType.PtyResizeRequested,
  terminalId,
  cols,
  rows
})
