import type {
  CiSnapshotWire,
  ReactionAggregateWire,
  RosterMemberWire,
  StateDeltaPayload,
  StateSnapshotPayload
} from './types'

export interface MirrorState {
  reactions: ReadonlyMap<string, ReactionAggregateWire[]>
  ci: CiSnapshotWire | null
  roster: RosterMemberWire[]
}

export const emptyMirrorState = (): MirrorState => ({
  reactions: new Map(),
  ci: null,
  roster: []
})

export const applyStateSnapshot = (
  _prev: MirrorState,
  snapshot: StateSnapshotPayload
): MirrorState => {
  const reactions = new Map<string, ReactionAggregateWire[]>()
  for (const entry of snapshot.reactions) {
    reactions.set(entry.channel, entry.aggregated)
  }
  return {
    reactions,
    ci: snapshot.ci ?? null,
    roster: snapshot.roster
  }
}

export const applyStateDelta = (prev: MirrorState, delta: StateDeltaPayload): MirrorState => {
  switch (delta.domain) {
    case 'reactions': {
      const reactions = new Map(prev.reactions)
      if (delta.aggregated.length === 0) reactions.delete(delta.channel)
      else reactions.set(delta.channel, delta.aggregated)
      return { ...prev, reactions }
    }
    case 'ci':
      return { ...prev, ci: delta.snapshot }
    case 'roster':
      return { ...prev, roster: delta.members }
    case 'iteration_close':
      return prev
    case 'iteration_close_blocked':
      return prev
  }
}
