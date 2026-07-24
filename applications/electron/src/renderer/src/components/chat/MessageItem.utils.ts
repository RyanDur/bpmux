import { format } from 'date-fns'
import type { ChatMessage } from '@transport/types'

export const formatTime = (ts: string): string => {
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return format(d, 'h:mm a zzz')
}

export const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

export const avatarClasses = (sender: string): string => {
  const shape = sender === 'owner' ? 'message-avatar--circle' : 'message-avatar--square'
  return `message-avatar ${shape}`
}

export const messageKey = (m: ChatMessage): string => `${m.ts}|${m.from}`
