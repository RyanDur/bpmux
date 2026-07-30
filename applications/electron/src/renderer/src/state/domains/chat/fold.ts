import { asMessageType } from '@transport/messageType'
import { ChatActionType, type HistoryMessage } from './actions'
import type { StateAction } from '../../actions'
import type { ChatMessage } from '@transport/types'

export type ChatState = {
  messages: ChatMessage[]
}

export const initialChat: ChatState = { messages: [] }

const messageKey = (message: { from: string; text: string; ts: string }): string =>
  `${message.ts}|${message.from}|${message.text}`

const mergeHistory = (state: ChatState, history: HistoryMessage[]): ChatMessage[] => {
  const firstBearer = (entry: HistoryMessage): boolean =>
    history.findIndex((other) => messageKey(other) === messageKey(entry)) === history.indexOf(entry)
  const hydrated = history.filter(firstBearer).map(
    (entry, position): ChatMessage => ({
      id: `hist-${entry.ts}-${position}`,
      from: entry.from,
      text: entry.text,
      ts: entry.ts,
      to: entry.to ?? null,
      addressee: entry.addressee ?? null,
      messageType: asMessageType(entry.message_type)
    })
  )
  const unseen = state.messages.filter(
    (message) => !hydrated.some((chat) => messageKey(chat) === messageKey(message))
  )
  return [...hydrated, ...unseen]
}

export const chatFold = (state: ChatState = initialChat, action: StateAction): ChatState => {
  switch (action.type) {
    case ChatActionType.MessageReceived: {
      const key = messageKey(action.message)
      if (state.messages.some((existing) => messageKey(existing) === key)) return state
      return { messages: [...state.messages, action.message] }
    }
    case ChatActionType.HistoryLoaded:
      return { messages: mergeHistory(state, action.history) }
    default:
      return state
  }
}
