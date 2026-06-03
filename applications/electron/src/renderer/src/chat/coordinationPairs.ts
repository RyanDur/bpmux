import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { shortName } from './MessageItem.utils'

export interface CoordinationPair {
  agentA: string
  agentB: string
  lastTs: string
  count: number
}

const BROADCAST = 'all'

const agentNamesOf = (roster: readonly RosterMemberWire[]): Set<string> => {
  const names = new Set<string>()
  for (const member of roster) {
    if (member.role !== 'Owner') names.add(shortName(member.name))
  }
  return names
}

const directedTargetsOf = (m: ChatMessage): readonly string[] => {
  if (m.addressee && m.addressee !== BROADCAST) return [m.addressee]
  if (m.to && m.to.length > 0 && !m.to.includes(BROADCAST)) return m.to
  return []
}

export const deriveCoordinationPairs = (
  messages: readonly ChatMessage[],
  roster: readonly RosterMemberWire[]
): readonly CoordinationPair[] => {
  const agents = agentNamesOf(roster)
  const byPair = new Map<string, CoordinationPair>()
  for (const m of messages) {
    const sender = shortName(m.from)
    if (!agents.has(sender)) continue
    for (const rawTarget of directedTargetsOf(m)) {
      const target = shortName(rawTarget)
      if (target === sender || !agents.has(target)) continue
      const [agentA, agentB] = sender < target ? [sender, target] : [target, sender]
      const key = [agentA, agentB].join('|')
      const existing = byPair.get(key)
      if (existing) {
        existing.count += 1
        if (m.ts > existing.lastTs) existing.lastTs = m.ts
      } else {
        byPair.set(key, { agentA, agentB, lastTs: m.ts, count: 1 })
      }
    }
  }
  return Array.from(byPair.values()).sort(
    (x, y) =>
      y.lastTs.localeCompare(x.lastTs) ||
      y.count - x.count ||
      x.agentA.localeCompare(y.agentA) ||
      x.agentB.localeCompare(y.agentB)
  )
}
