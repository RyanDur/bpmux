import { asMessageType } from '@transport/messageType'
import { StateActionType, type HistoryMessage, type StateAction } from '../../actions'
import type { ChatMessage } from '@transport/types'

export type ChatSlice = {
  messages: ChatMessage[]
}

export const emptyChat = (): ChatSlice => ({ messages: [] })

const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

const mergeHistory = (slice: ChatSlice, history: HistoryMessage[]): ChatMessage[] => {
  const seen = new Set<string>()
  const merged: ChatMessage[] = []
  for (const h of history) {
    const chat: ChatMessage = {
      id: `hist-${h.ts}-${merged.length}`,
      from: h.from,
      text: h.text,
      ts: h.ts,
      to: h.to ?? null,
      addressee: h.addressee ?? null,
      messageType: asMessageType(h.message_type)
    }
    const k = messageKey(chat)
    if (seen.has(k)) continue
    seen.add(k)
    merged.push(chat)
  }
  for (const m of slice.messages) {
    const k = messageKey(m)
    if (seen.has(k)) continue
    seen.add(k)
    merged.push(m)
  }
  return merged
}

export const chatFold = (slice: ChatSlice, action: StateAction): ChatSlice | null => {
  switch (action.type) {
    case StateActionType.MessageReceived: {
      const key = messageKey(action.message)
      if (slice.messages.some((m) => messageKey(m) === key)) return null
      return { messages: [...slice.messages, action.message] }
    }
    case StateActionType.HistoryLoaded:
      return { messages: mergeHistory(slice, action.history) }
    default:
      return null
  }
}
