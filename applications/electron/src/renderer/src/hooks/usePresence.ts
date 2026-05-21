import { useMemo } from 'react'
import type { ChatMessage } from '../transport/types'

// Presence states mirroring the set_status hook vocabulary. bpmux
// hooks fire these values via set_status as activity-class broadcast
// messages — this hook just derives the most-recent-per-role state
// from the log. Unknown / missing = 'offline'.
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

/**
 * Derive each role's current presence from activity-class messages in
 * the log. Walks the messages stream once in reverse (newest first),
 * picking the first activity match per role. Complexity O(N), fine
 * for chat-log scale; could cache per new-message if we see it hot.
 */
export const usePresence = (
  messages: readonly ChatMessage[],
  roles: readonly string[]
): Map<string, Presence> =>
  useMemo(() => {
    const out = new Map<string, Presence>()
    for (const role of roles) out.set(role, 'offline')

    // Iterate newest-first for early exit once each role is filled.
    for (let i = messages.length - 1; i >= 0; i--) {
      const m = messages[i]
      if (m.messageType !== 'activity') continue
      const role = shortName(m.from)
      if (!roles.includes(role)) continue
      // Only set if we haven't already recorded the newest activity
      // for this role.
      if (out.get(role) !== 'offline') continue
      const p = asPresence(m.text)
      if (p) out.set(role, p)
      // Early exit when all roles filled.
      if ([...out.values()].every((v) => v !== 'offline')) break
    }
    return out
  }, [messages, roles])
