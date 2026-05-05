import type { ChatMessage } from '../transport/types'

export const newMessagesContainMarker = (
  prevSeenCount: number,
  messages: ChatMessage[],
  marker: string
): boolean => {
  if (messages.length <= prevSeenCount) return false
  for (let i = prevSeenCount; i < messages.length; i++) {
    if (messages[i].text === marker) return true
  }
  return false
}
