import { useMemo } from 'react'
import type { ChatMessage } from '../transport/types'

export type Presence = 'available' | 'busy' | 'blocked' | 'idle' | 'help' | 'offline'

const KNOWN: readonly Presence[] = ['available', 'busy', 'blocked', 'idle', 'help', 'offline']

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

const asPresence = (text: string): Presence | null => {
  const t = text.trim().toLowerCase()
  return (KNOWN as readonly string[]).includes(t) ? (t as Presence) : null
}

export const usePresence = (
  messages: readonly ChatMessage[],
  roles: readonly string[]
): Map<string, Presence> =>
  useMemo(() => {
    const out = new Map<string, Presence>()
    for (const role of roles) out.set(role, 'offline')

    for (let i = messages.length - 1; i >= 0; i--) {
      const m = messages[i]
      if (m.messageType !== 'activity') continue
      const role = shortName(m.from)
      if (!roles.includes(role)) continue
      if (out.get(role) !== 'offline') continue
      const p = asPresence(m.text)
      if (p) out.set(role, p)
      if ([...out.values()].every((v) => v !== 'offline')) break
    }
    return out
  }, [messages, roles])
