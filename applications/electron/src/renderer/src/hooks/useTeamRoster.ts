import type { PresenceWire, RosterMemberWire } from '../transport/types'
import { PRESENCE, type Presence } from './usePresence'

export interface RosterEntry {
  identity: string
  role: string
  presence: Presence
}

const WIRE_TO_PRESENCE: Record<PresenceWire, Presence> = {
  Online: PRESENCE.Present,
  Busy: PRESENCE.Busy,
  Idle: PRESENCE.Idle,
  Offline: PRESENCE.Offline,
  Blocked: PRESENCE.Blocked,
  Help: PRESENCE.Help
}

const presenceFromWire = (presence: PresenceWire | undefined): Presence =>
  presence === undefined ? PRESENCE.Unknown : (WIRE_TO_PRESENCE[presence] ?? PRESENCE.Unknown)

export const wireToRosterEntry = (wire: RosterMemberWire): RosterEntry => ({
  identity: wire.identity,
  role: wire.name,
  presence: presenceFromWire(wire.presence)
})

interface Props {
  mirrorRoster?: readonly RosterMemberWire[]
}

export const useTeamRoster = ({ mirrorRoster }: Props): readonly RosterEntry[] => {
  if (!mirrorRoster || mirrorRoster.length === 0) return []
  return mirrorRoster.map(wireToRosterEntry)
}
