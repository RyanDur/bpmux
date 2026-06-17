import type { RosterMemberWire } from '../transport/types'
import { presenceFromWire, type Presence } from './usePresence'

export interface RosterEntry {
  identity: string
  role: string
  presence: Presence
}

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
