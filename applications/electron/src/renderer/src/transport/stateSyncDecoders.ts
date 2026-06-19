import * as D from 'schemawax'
import {
  type CiSnapshotWire,
  type ReactionAggregateWire,
  type ReactionsChannelSnapshot,
  type RosterMemberWire,
  type StateDeltaPayload,
  type StateSnapshotPayload,
  type SyncStatusWire
} from './types'

const ciState = D.literalUnion('unknown', 'green', 'red', 'running', 'quota_exhausted')

const ciSnapshot: D.Decoder<CiSnapshotWire> = D.object({
  required: { state: ciState },
  optional: {
    run_id: D.nullable(D.number),
    head_sha: D.nullable(D.string),
    display_title: D.nullable(D.string),
    last_successful_poll_at: D.nullable(D.string),
    url: D.nullable(D.string)
  }
})

const droppedEvent = D.object({
  required: { story_number: D.number, error: D.string, ts: D.string }
})

const syncStatus: D.Decoder<SyncStatusWire> = D.object({
  required: {
    last_ingest_at: D.nullable(D.string),
    last_push_at: D.nullable(D.string),
    pending_events: D.number,
    last_error: D.nullable(D.string),
    dropped_events: D.array(droppedEvent),
    recovering_from_corruption: D.boolean,
    stale: D.boolean,
    startup_fail: D.boolean,
    conflicts_pending: D.number
  }
})

const rosterMember: D.Decoder<RosterMemberWire> = D.object({
  required: {
    routing_id: D.string,
    name: D.string,
    identity: D.string,
    role: D.literalUnion('Designer', 'Dev', 'Pm', 'Owner', 'Other'),
    presence: D.literalUnion('Online', 'Busy', 'Idle', 'Offline', 'Blocked', 'Help')
  }
})

const reactionAggregate: D.Decoder<ReactionAggregateWire> = D.object({
  required: {
    message_key: D.string,
    emoji: D.string,
    count: D.number,
    users: D.array(D.string)
  }
})

const reactionsChannelSnapshot: D.Decoder<ReactionsChannelSnapshot> = D.object({
  required: { channel: D.string, aggregated: D.array(reactionAggregate) }
})

export const stateSnapshotDecoder: D.Decoder<StateSnapshotPayload> = D.object({
  required: {
    reactions: D.array(reactionsChannelSnapshot),
    roster: D.array(rosterMember)
  },
  optional: { ci: D.nullable(ciSnapshot) }
})

export const stateDeltaDecoder: D.Decoder<StateDeltaPayload> = D.oneOf(
  D.object({
    required: {
      domain: D.literal('reactions'),
      channel: D.string,
      aggregated: D.array(reactionAggregate)
    }
  }),
  D.object({ required: { domain: D.literal('ci'), snapshot: ciSnapshot } }),
  D.object({ required: { domain: D.literal('ci_poll'), snapshot: ciSnapshot } }),
  D.object({ required: { domain: D.literal('sync_poll'), status: syncStatus } }),
  D.object({ required: { domain: D.literal('roster'), members: D.array(rosterMember) } }),
  D.object({
    required: {
      domain: D.literal('iteration_close'),
      promoted: D.array(D.number),
      deleted_marker_id: D.nullable(D.number)
    }
  }),
  D.object({
    required: {
      domain: D.literal('iteration_close_blocked'),
      blockers: D.array(
        D.object({ required: { number: D.number, title: D.string, state: D.string } })
      )
    }
  }),
  D.object({ required: { domain: D.literal('board_invalidated'), reason: D.string } }),
  D.object({ required: { domain: D.literal('board_snapshot'), stories: D.array(D.unknown) } })
)
