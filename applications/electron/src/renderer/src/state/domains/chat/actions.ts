import type { ChatMessage } from '@transport/types'

export enum ChatActionType {
  MessageReceived = 'state/messageReceived',
  HistoryLoaded = 'state/historyLoaded',
  ChatSendRequested = 'state/chatSendRequested',
  ReactionToggleRequested = 'state/reactionToggleRequested',
  OlderHistoryRequested = 'state/olderHistoryRequested'
}

export type SendOptions = {
  addressee?: string | null
  to?: string[] | null
}

export type HistoryMessage = {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

export type ChatAction =
  | { type: ChatActionType.MessageReceived; message: ChatMessage }
  | { type: ChatActionType.HistoryLoaded; history: HistoryMessage[] }
  | { type: ChatActionType.ChatSendRequested; text: string; opts?: SendOptions }
  | {
      type: ChatActionType.ReactionToggleRequested
      channel: string
      messageKey: string
      emoji: string
      user: string
    }
  | {
      type: ChatActionType.OlderHistoryRequested
      requestId: number
      before: string
      limit: number
    }

export const messageReceived = (message: ChatMessage): ChatAction => ({
  type: ChatActionType.MessageReceived,
  message
})

export const historyLoaded = (history: HistoryMessage[]): ChatAction => ({
  type: ChatActionType.HistoryLoaded,
  history
})

export const chatSendRequested = (text: string, opts?: SendOptions): ChatAction => ({
  type: ChatActionType.ChatSendRequested,
  text,
  opts
})

export const reactionToggleRequested = (
  channel: string,
  messageKey: string,
  emoji: string,
  user: string
): ChatAction => ({
  type: ChatActionType.ReactionToggleRequested,
  channel,
  messageKey,
  emoji,
  user
})

export const olderHistoryRequested = (
  requestId: number,
  before: string,
  limit: number
): ChatAction => ({
  type: ChatActionType.OlderHistoryRequested,
  requestId,
  before,
  limit
})
