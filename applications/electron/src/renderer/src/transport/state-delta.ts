import type {
  CiSnapshotWire,
  ReactionAggregateWire,
  RosterMemberWire,
  StateDeltaPayload,
  SyncStatusWire
} from './types'

export type MirrorState = {
  reactions: ReadonlyMap<string, ReactionAggregateWire[]>
  ci: CiSnapshotWire | null
  sync: SyncStatusWire | null
  roster: RosterMemberWire[]
}

export const emptyMirrorState = (): MirrorState => ({
  reactions: new Map(),
  ci: null,
  sync: null,
  roster: []
})

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
    case 'ci_poll':
      return { ...prev, ci: delta.snapshot }
    case 'sync_poll':
      return { ...prev, sync: delta.status }
    case 'roster':
      return { ...prev, roster: delta.members }
    default:
      return prev
  }
}
