import { caseOf } from '@ryandur/sand'
import { asMessageType } from '@transport/messageType'
import { StateActionType, type HistoryMessage, type StateAction } from '../../actions'
import type { ChatMessage } from '@transport/types'

export type ChatSlice = {
  messages: ChatMessage[]
}

export const emptyChat = (): ChatSlice => ({ messages: [] })

const messageKey = (message: { from: string; text: string; ts: string }): string =>
  `${message.ts}|${message.from}|${message.text}`

const mergeHistory = (slice: ChatSlice, history: HistoryMessage[]): ChatMessage[] => {
  const seen = new Set<string>()
  const merged: ChatMessage[] = []
  for (const entry of history) {
    const chat: ChatMessage = {
      id: `hist-${entry.ts}-${merged.length}`,
      from: entry.from,
      text: entry.text,
      ts: entry.ts,
      to: entry.to ?? null,
      addressee: entry.addressee ?? null,
      messageType: asMessageType(entry.message_type)
    }
    const key = messageKey(chat)
    if (seen.has(key)) continue
    seen.add(key)
    merged.push(chat)
  }
  for (const message of slice.messages) {
    const key = messageKey(message)
    if (seen.has(key)) continue
    seen.add(key)
    merged.push(message)
  }
  return merged
}

const onActionType = caseOf('type')

export const chatFold = (slice: ChatSlice, action: StateAction): ChatSlice | null =>
  onActionType<StateAction, ChatSlice | null>(action, {
    [StateActionType.MessageReceived]: ({ message }) => {
      const key = messageKey(message)
      if (slice.messages.some((existing) => messageKey(existing) === key)) return null
      return { messages: [...slice.messages, message] }
    },
    [StateActionType.HistoryLoaded]: ({ history }) => ({
      messages: mergeHistory(slice, history)
    })
  }).orNull()
