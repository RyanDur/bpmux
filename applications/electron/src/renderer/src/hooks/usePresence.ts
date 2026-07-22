import { useMemo } from 'react'
import type { PresenceWire } from '../transport/types'
import { useRoster } from '../state'

export const PRESENCE = {
  Busy: 'busy',
  Blocked: 'blocked',
  Idle: 'idle',
  Help: 'help',
  Offline: 'offline',
  Unknown: 'unknown'
} as const

export type Presence = (typeof PRESENCE)[keyof typeof PRESENCE]

const WIRE_TO_PRESENCE: Record<PresenceWire, Presence> = {
  Online: PRESENCE.Unknown,
  Busy: PRESENCE.Busy,
  Idle: PRESENCE.Idle,
  Offline: PRESENCE.Offline,
  Blocked: PRESENCE.Blocked,
  Help: PRESENCE.Help
}

export const presenceFromWire = (presence: PresenceWire | undefined): Presence =>
  presence === undefined ? PRESENCE.Unknown : (WIRE_TO_PRESENCE[presence] ?? PRESENCE.Unknown)

export const usePresence = (): Map<string, Presence> => {
  const roster = useRoster()
  return useMemo(() => {
    const out = new Map<string, Presence>()
    for (const wire of roster) out.set(wire.name, presenceFromWire(wire.presence))
    return out
  }, [roster])
}
