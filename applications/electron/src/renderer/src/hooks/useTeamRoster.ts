import type { RosterMemberWire } from '../transport/types'

export interface RosterEntry {
  identity: string
  role: string
}

export const wireToRosterEntry = (wire: RosterMemberWire): RosterEntry => ({
  identity: wire.identity,
  role: wire.name
})

interface Props {
  mirrorRoster?: readonly RosterMemberWire[]
}

export const useTeamRoster = ({ mirrorRoster }: Props): readonly RosterEntry[] => {
  if (!mirrorRoster || mirrorRoster.length === 0) return []
  return mirrorRoster.map(wireToRosterEntry)
}
