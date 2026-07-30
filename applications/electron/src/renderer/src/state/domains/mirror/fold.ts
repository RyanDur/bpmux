import { caseOf } from '@ryandur/sand'
import { StateActionType, type StateAction } from '../../actions'
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

const onActionType = caseOf('type')
const onDomain = caseOf('domain')

export const mirrorFold = (
  state: { mirror: MirrorState } = { mirror: initialMirror },
  action: StateAction
): { mirror: MirrorState } =>
  onActionType<StateAction, { mirror: MirrorState }>(action, {
    [StateActionType.MirrorDeltaReceived]: ({ delta }) => {
      const mirror = applyStateDelta(state.mirror, delta)
      return mirror === state.mirror ? state : { mirror }
    }
  }).orElse(state)

export const applyStateDelta = (prev: MirrorState, delta: StateDeltaPayload): MirrorState =>
  onDomain<StateDeltaPayload, MirrorState>(delta, {
    reaction_updated: ({ channel, message_key, emoji, count, users }) => {
      const list = prev.reactions[channel] ?? []
      const index = list.findIndex(
        (aggregate) => aggregate.message_key === message_key && aggregate.emoji === emoji
      )
      const aggregate = { message_key, emoji, count, users }
      const nextList =
        count === 0
          ? list.filter((_existing, position) => position !== index)
          : index === -1
            ? [...list, aggregate]
            : list.map((existing, position) => (position === index ? aggregate : existing))
      return { ...prev, reactions: keep(prev.reactions, channel, nextList) }
    },
    roster_member_upserted: ({ member }) => {
      const index = prev.roster.findIndex((existing) => existing.routing_id === member.routing_id)
      const roster =
        index === -1
          ? [...prev.roster, member]
          : prev.roster.map((existing, position) => (position === index ? member : existing))
      return { ...prev, roster }
    },
    roster_member_removed: ({ routing_id }) => ({
      ...prev,
      roster: prev.roster.filter((member) => member.routing_id !== routing_id)
    }),
    reactions: ({ channel, aggregated }) => ({
      ...prev,
      reactions: keep(prev.reactions, channel, aggregated)
    }),
    ci: ({ snapshot }) => ({ ...prev, ci: snapshot }),
    ci_poll: ({ snapshot }) => ({ ...prev, ci: snapshot }),
    sync_poll: ({ status }) => ({ ...prev, sync: status }),
    roster: ({ members }) => ({ ...prev, roster: members })
  }).orElse(prev)
