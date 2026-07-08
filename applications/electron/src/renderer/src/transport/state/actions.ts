import type {
  ChatMessage,
  CommentWire,
  StateDeltaPayload,
  StateSnapshotPayload,
  TimelineEventWire
} from '../types'
export interface SendOptions {
  addressee?: string | null
  to?: string[] | null
}

export interface ConnectionConfig {
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

export const StateActionType = {
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
  PtyModeReceived: 'session/ptyModeReceived',
  StartRequested: 'session/startRequested',
  ChatSendRequested: 'session/chatSendRequested',
  ReactionToggleRequested: 'session/reactionToggleRequested',
  CommentsHydrateRequested: 'session/commentsHydrateRequested',
  CommentsHydrated: 'session/commentsHydrated',
  TimelineHydrateRequested: 'session/timelineHydrateRequested',
  TimelineHydrated: 'session/timelineHydrated',
  HistoryHydrateRequested: 'session/historyHydrateRequested',
  OlderHistoryRequested: 'session/olderHistoryRequested',
  OutcomeHorizonChangeRequested: 'session/outcomeHorizonChangeRequested',
  OutcomeIterationReorderRequested: 'session/outcomeIterationReorderRequested',
  OutcomeReprioritizeRequested: 'session/outcomeReprioritizeRequested',
  IterationReorderRequested: 'session/iterationReorderRequested',
  AddStoryToIterationRequested: 'session/addStoryToIterationRequested',
  PtySubscribeRequested: 'session/ptySubscribeRequested',
  PtyInputRequested: 'session/ptyInputRequested',
  PtyResizeRequested: 'session/ptyResizeRequested',
  FrameReceived: 'session/frameReceived',
  Reset: 'session/reset'
} as const

export type StateAction =
  | { type: typeof StateActionType.Connecting }
  | { type: typeof StateActionType.Connected }
  | { type: typeof StateActionType.Reconnected }
  | { type: typeof StateActionType.Disconnected }
  | { type: typeof StateActionType.SessionIdentified; sessionId: string }
  | { type: typeof StateActionType.MessageReceived; message: ChatMessage }
  | { type: typeof StateActionType.HistoryLoaded; history: HistoryMessage[] }
  | { type: typeof StateActionType.MirrorSnapshotReceived; payload: StateSnapshotPayload }
  | { type: typeof StateActionType.MirrorDeltaReceived; delta: StateDeltaPayload; ts: string }
  | { type: typeof StateActionType.PtyOutputReceived; terminalId: string; data: string }
  | { type: typeof StateActionType.PtyExitReceived; terminalId: string }
  | {
      type: typeof StateActionType.PtyModeReceived
      terminalId: string
      bracketedPaste: boolean
    }
  | { type: typeof StateActionType.StartRequested; config: ConnectionConfig }
  | { type: typeof StateActionType.ChatSendRequested; text: string; opts?: SendOptions }
  | {
      type: typeof StateActionType.ReactionToggleRequested
      channel: string
      messageKey: string
      emoji: string
      user: string
    }
  | { type: typeof StateActionType.CommentsHydrateRequested; storyId: number }
  | {
      type: typeof StateActionType.CommentsHydrated
      storyId: number
      comments: CommentWire[]
    }
  | { type: typeof StateActionType.TimelineHydrateRequested; storyId: number }
  | {
      type: typeof StateActionType.TimelineHydrated
      storyId: number
      events: TimelineEventWire[]
    }
  | {
      type: typeof StateActionType.HistoryHydrateRequested
      chatLimit: number
      activityLimit: number
    }
  | {
      type: typeof StateActionType.OlderHistoryRequested
      requestId: number
      before: string
      limit: number
    }
  | {
      type: typeof StateActionType.OutcomeHorizonChangeRequested
      outcomeId: number
      horizon: 'now' | 'next' | 'later'
    }
  | {
      type: typeof StateActionType.OutcomeIterationReorderRequested
      outcomeId: number
      iterationId: number
      position: number
    }
  | {
      type: typeof StateActionType.OutcomeReprioritizeRequested
      outcomeId: number
      position: number
    }
  | {
      type: typeof StateActionType.IterationReorderRequested
      iterationId: number
      position: number
    }
  | {
      type: typeof StateActionType.AddStoryToIterationRequested
      iterationId: number
      storyId: number
      position: number
    }
  | { type: typeof StateActionType.PtySubscribeRequested; terminalId: string }
  | { type: typeof StateActionType.PtyInputRequested; terminalId: string; keystroke: string }
  | {
      type: typeof StateActionType.PtyResizeRequested
      terminalId: string
      cols: number
      rows: number
    }
  | { type: typeof StateActionType.FrameReceived; raw: string }
  | { type: typeof StateActionType.Reset }

export const connecting = (): StateAction => ({ type: StateActionType.Connecting })
export const connected = (): StateAction => ({ type: StateActionType.Connected })
export const reconnected = (): StateAction => ({ type: StateActionType.Reconnected })
export const disconnected = (): StateAction => ({ type: StateActionType.Disconnected })

export const sessionIdentified = (sessionId: string): StateAction => ({
  type: StateActionType.SessionIdentified,
  sessionId
})

export const messageReceived = (message: ChatMessage): StateAction => ({
  type: StateActionType.MessageReceived,
  message
})

export const historyLoaded = (history: HistoryMessage[]): StateAction => ({
  type: StateActionType.HistoryLoaded,
  history
})

export const mirrorSnapshotReceived = (payload: StateSnapshotPayload): StateAction => ({
  type: StateActionType.MirrorSnapshotReceived,
  payload
})

export const mirrorDeltaReceived = (delta: StateDeltaPayload, ts: string): StateAction => ({
  type: StateActionType.MirrorDeltaReceived,
  delta,
  ts
})

export const ptyOutputReceived = (terminalId: string, data: string): StateAction => ({
  type: StateActionType.PtyOutputReceived,
  terminalId,
  data
})

export const ptyExitReceived = (terminalId: string): StateAction => ({
  type: StateActionType.PtyExitReceived,
  terminalId
})

export const ptyModeReceived = (terminalId: string, bracketedPaste: boolean): StateAction => ({
  type: StateActionType.PtyModeReceived,
  terminalId,
  bracketedPaste
})

export const startRequested = (config: ConnectionConfig): StateAction => ({
  type: StateActionType.StartRequested,
  config
})

export const chatSendRequested = (text: string, opts?: SendOptions): StateAction => ({
  type: StateActionType.ChatSendRequested,
  text,
  opts
})

export const reactionToggleRequested = (
  channel: string,
  messageKey: string,
  emoji: string,
  user: string
): StateAction => ({
  type: StateActionType.ReactionToggleRequested,
  channel,
  messageKey,
  emoji,
  user
})

export const commentsHydrateRequested = (storyId: number): StateAction => ({
  type: StateActionType.CommentsHydrateRequested,
  storyId
})

export const commentsHydrated = (storyId: number, comments: CommentWire[]): StateAction => ({
  type: StateActionType.CommentsHydrated,
  storyId,
  comments
})

export const timelineHydrateRequested = (storyId: number): StateAction => ({
  type: StateActionType.TimelineHydrateRequested,
  storyId
})

export const timelineHydrated = (storyId: number, events: TimelineEventWire[]): StateAction => ({
  type: StateActionType.TimelineHydrated,
  storyId,
  events
})

export const historyHydrateRequested = (chatLimit: number, activityLimit: number): StateAction => ({
  type: StateActionType.HistoryHydrateRequested,
  chatLimit,
  activityLimit
})

export const olderHistoryRequested = (
  requestId: number,
  before: string,
  limit: number
): StateAction => ({
  type: StateActionType.OlderHistoryRequested,
  requestId,
  before,
  limit
})

export const outcomeHorizonChangeRequested = (
  outcomeId: number,
  horizon: 'now' | 'next' | 'later'
): StateAction => ({
  type: StateActionType.OutcomeHorizonChangeRequested,
  outcomeId,
  horizon
})

export const outcomeIterationReorderRequested = (
  outcomeId: number,
  iterationId: number,
  position: number
): StateAction => ({
  type: StateActionType.OutcomeIterationReorderRequested,
  outcomeId,
  iterationId,
  position
})

export const outcomeReprioritizeRequested = (outcomeId: number, position: number): StateAction => ({
  type: StateActionType.OutcomeReprioritizeRequested,
  outcomeId,
  position
})

export const iterationReorderRequested = (iterationId: number, position: number): StateAction => ({
  type: StateActionType.IterationReorderRequested,
  iterationId,
  position
})

export const addStoryToIterationRequested = (
  iterationId: number,
  storyId: number,
  position: number
): StateAction => ({
  type: StateActionType.AddStoryToIterationRequested,
  iterationId,
  storyId,
  position
})

export const ptySubscribeRequested = (terminalId: string): StateAction => ({
  type: StateActionType.PtySubscribeRequested,
  terminalId
})

export const ptyInputRequested = (terminalId: string, keystroke: string): StateAction => ({
  type: StateActionType.PtyInputRequested,
  terminalId,
  keystroke
})

export const ptyResizeRequested = (
  terminalId: string,
  cols: number,
  rows: number
): StateAction => ({
  type: StateActionType.PtyResizeRequested,
  terminalId,
  cols,
  rows
})

export const frameReceived = (raw: string): StateAction => ({
  type: StateActionType.FrameReceived,
  raw
})

export const reset = (): StateAction => ({ type: StateActionType.Reset })
