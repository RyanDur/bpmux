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

const onActionType = caseOf('type')
const onDomain = caseOf('domain')

export const mirrorFold = (
  slice: { mirror: MirrorState },
  action: StateAction
): { mirror: MirrorState } | null =>
  onActionType<StateAction, { mirror: MirrorState } | null>(action, {
    [StateActionType.MirrorDeltaReceived]: ({ delta }) => {
      const mirror = applyStateDelta(slice.mirror, delta)
      return mirror === slice.mirror ? null : { mirror }
    }
  }).orNull()

export const applyStateDelta = (prev: MirrorState, delta: StateDeltaPayload): MirrorState =>
  onDomain<StateDeltaPayload, MirrorState>(delta, {
    reaction_updated: ({ channel, message_key, emoji, count, users }) => {
      const reactions = new Map(prev.reactions)
      const list = reactions.get(channel) ?? []
      const index = list.findIndex(
        (aggregate) => aggregate.message_key === message_key && aggregate.emoji === emoji
      )
      const aggregate = { message_key, emoji, count, users }
      const nextList =
        count === 0
          ? list.filter((_, position) => position !== index)
          : index === -1
            ? [...list, aggregate]
            : list.map((existing, position) => (position === index ? aggregate : existing))
      if (nextList.length === 0) reactions.delete(channel)
      else reactions.set(channel, nextList)
      return { ...prev, reactions }
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
    reactions: ({ channel, aggregated }) => {
      const reactions = new Map(prev.reactions)
      if (aggregated.length === 0) reactions.delete(channel)
      else reactions.set(channel, aggregated)
      return { ...prev, reactions }
    },
    ci: ({ snapshot }) => ({ ...prev, ci: snapshot }),
    ci_poll: ({ snapshot }) => ({ ...prev, ci: snapshot }),
    sync_poll: ({ status }) => ({ ...prev, sync: status }),
    roster: ({ members }) => ({ ...prev, roster: members })
  }).orElse(prev)
