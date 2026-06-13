import { useMemo } from 'react'
import { MESSAGE_TYPE } from '../transport/messageType'
import type { ChatMessage } from '../transport/types'

export const PRESENCE = {
  Available: 'available',
  Present: 'present',
  Busy: 'busy',
  Blocked: 'blocked',
  Idle: 'idle',
  Help: 'help',
  Offline: 'offline',
  Unknown: 'unknown'
} as const

export type Presence = (typeof PRESENCE)[keyof typeof PRESENCE]

const KNOWN: readonly string[] = Object.values(PRESENCE)

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

const asPresence = (text: string): Presence | null => {
  const t = text.trim().toLowerCase()
  return KNOWN.includes(t) ? (t as Presence) : null
}

export const usePresence = (
  messages: readonly ChatMessage[],
  roles: readonly string[]
): Map<string, Presence> =>
  useMemo(() => {
    const out = new Map<string, Presence>()
    for (const role of roles) out.set(role, PRESENCE.Offline)

    for (let i = messages.length - 1; i >= 0; i--) {
      const m = messages[i]
      if (m.messageType !== MESSAGE_TYPE.Activity) continue
      const role = shortName(m.from)
      if (!roles.includes(role)) continue
      if (out.get(role) !== PRESENCE.Offline) continue
      const p = asPresence(m.text)
      if (p) out.set(role, p)
      if ([...out.values()].every((v) => v !== PRESENCE.Offline)) break
    }
    return out
  }, [messages, roles])
