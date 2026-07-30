import { MirrorActionType } from './actions'
import type { StateAction } from '../../actions'
import type {
  CiSnapshotWire,
  ReactionAggregateWire,
  RosterMemberWire,
  StateDeltaPayload,
  SyncStatusWire
} from '@transport/types'

export type MirrorState = {
  reactions: Readonly<Record<string, ReactionAggregateWire[]>>
  ci: CiSnapshotWire | null
  sync: SyncStatusWire | null
  roster: RosterMemberWire[]
}

export const initialMirror: MirrorState = {
  reactions: {},
  ci: null,
  sync: null,
  roster: []
}

const keep = (
  reactions: MirrorState['reactions'],
  channel: string,
  aggregates: ReactionAggregateWire[]
): MirrorState['reactions'] => {
  const { [channel]: _replaced, ...others } = reactions
  return aggregates.length === 0 ? others : { ...others, [channel]: aggregates }
}

export const mirrorFold = (
  state: { mirror: MirrorState } = { mirror: initialMirror },
  action: StateAction
): { mirror: MirrorState } => {
  if (action.type !== MirrorActionType.MirrorDeltaReceived) return state
  const mirror = applyStateDelta(state.mirror, action.delta)
  return mirror === state.mirror ? state : { mirror }
}

export const applyStateDelta = (prev: MirrorState, delta: StateDeltaPayload): MirrorState => {
  switch (delta.domain) {
    case 'reaction_updated': {
      const list = prev.reactions[delta.channel] ?? []
      const index = list.findIndex(
        (aggregate) =>
          aggregate.message_key === delta.message_key && aggregate.emoji === delta.emoji
      )
      const aggregate = {
        message_key: delta.message_key,
        emoji: delta.emoji,
        count: delta.count,
        users: delta.users
      }
      const nextList =
        delta.count === 0
          ? list.filter((_existing, position) => position !== index)
          : index === -1
            ? [...list, aggregate]
            : list.map((existing, position) => (position === index ? aggregate : existing))
      return { ...prev, reactions: keep(prev.reactions, delta.channel, nextList) }
    }
    case 'roster_member_upserted': {
      const index = prev.roster.findIndex(
        (existing) => existing.routing_id === delta.member.routing_id
      )
      const roster =
        index === -1
          ? [...prev.roster, delta.member]
          : prev.roster.map((existing, position) => (position === index ? delta.member : existing))
      return { ...prev, roster }
    }
    case 'roster_member_removed':
      return {
        ...prev,
        roster: prev.roster.filter((member) => member.routing_id !== delta.routing_id)
      }
    case 'reactions':
      return { ...prev, reactions: keep(prev.reactions, delta.channel, delta.aggregated) }
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
