import type { RosterMemberWire } from '../transport/types'
import { useRoster } from '../state'
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

export const useTeamRoster = (): readonly RosterEntry[] => useRoster().map(wireToRosterEntry)
