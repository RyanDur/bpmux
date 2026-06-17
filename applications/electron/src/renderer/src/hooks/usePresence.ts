import { useMemo } from 'react'
import type { PresenceWire, RosterMemberWire } from '../transport/types'

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

export const usePresence = (
  mirrorRoster: readonly RosterMemberWire[] | undefined
): Map<string, Presence> =>
  useMemo(() => {
    const out = new Map<string, Presence>()
    for (const wire of mirrorRoster ?? []) out.set(wire.name, presenceFromWire(wire.presence))
    return out
  }, [mirrorRoster])
