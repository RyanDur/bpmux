import { StateActionType, type StateAction } from '../../actions'
import type {
  CiSnapshotWire,
  ReactionAggregateWire,
  RosterMemberWire,
  StateDeltaPayload,
  SyncStatusWire
} from '@transport/types'

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

export const mirrorFold = (
  slice: { mirror: MirrorState },
  action: StateAction
): { mirror: MirrorState } | null => {
  if (action.type !== StateActionType.MirrorDeltaReceived) return null
  const mirror = applyStateDelta(slice.mirror, action.delta)
  return mirror === slice.mirror ? null : { mirror }
}

export const applyStateDelta = (prev: MirrorState, delta: StateDeltaPayload): MirrorState => {
  switch (delta.domain) {
    case 'reaction_updated': {
      const reactions = new Map(prev.reactions)
      const list = reactions.get(delta.channel) ?? []
      const index = list.findIndex(
        (a) => a.message_key === delta.message_key && a.emoji === delta.emoji
      )
      const aggregate = {
        message_key: delta.message_key,
        emoji: delta.emoji,
        count: delta.count,
        users: delta.users
      }
      const nextList =
        delta.count === 0
          ? list.filter((_, i) => i !== index)
          : index === -1
            ? [...list, aggregate]
            : list.map((a, i) => (i === index ? aggregate : a))
      if (nextList.length === 0) reactions.delete(delta.channel)
      else reactions.set(delta.channel, nextList)
      return { ...prev, reactions }
    }
    case 'roster_member_upserted': {
      const index = prev.roster.findIndex((m) => m.routing_id === delta.member.routing_id)
      const roster =
        index === -1
          ? [...prev.roster, delta.member]
          : prev.roster.map((m, i) => (i === index ? delta.member : m))
      return { ...prev, roster }
    }
    case 'roster_member_removed':
      return { ...prev, roster: prev.roster.filter((m) => m.routing_id !== delta.routing_id) }
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
