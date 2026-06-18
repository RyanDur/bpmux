import type { ChatMessage } from './types'

export const MAX_MESSAGES = 500
export const CAP_TRIGGER_THRESHOLD = MAX_MESSAGES * 2

export const capMessages = (messages: ChatMessage[], max: number): ChatMessage[] =>
  messages.length > max ? messages.slice(messages.length - max) : messages

export const capPerPush = (next: ChatMessage[], atBottom: boolean): ChatMessage[] => {
  if (!atBottom) return next
  if (next.length <= CAP_TRIGGER_THRESHOLD) return next
  return next.slice(next.length - MAX_MESSAGES)
}
