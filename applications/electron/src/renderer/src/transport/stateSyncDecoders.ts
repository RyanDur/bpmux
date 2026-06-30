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
import type { RoadmapCard, RoadmapOutcomeData } from '../board/selectRoadmap'
import type { BoardIterationData } from '../board/selectIterations'
import { story } from '../board/boardDecoder'

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
  required: { story_id: D.number, error: D.string, ts: D.string }
})

const syncStatus: D.Decoder<SyncStatusWire> = D.object({
  required: {
    last_ingest_at: D.nullable(D.string),
    last_push_at: D.nullable(D.string),
    pending_events: D.number,
    last_error: D.nullable(D.string),
    dropped_events: D.array(droppedEvent),
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

const outcomeReadinessWire = D.object({
  required: { iteration_count: D.number, gap_count: D.number, all_ready: D.boolean }
})

const iterationReadinessWire = D.object({
  required: {
    drafted: D.boolean,
    stories_ordered: D.boolean,
    review_closed: D.boolean,
    dev_ok: D.boolean
  }
})

const roadmapIteration = D.object({
  required: {
    id: D.number,
    name: D.string,
    position: D.number,
    readiness: iterationReadinessWire
  }
})

const boardIteration: D.Decoder<BoardIterationData> = D.object({
  required: {
    id: D.number,
    title: D.string,
    iteration_order: D.nullable(D.number),
    is_current: D.boolean,
    complete: D.boolean,
    readiness: iterationReadinessWire,
    stories: D.array(D.number)
  }
})

const roadmapOutcome: D.Decoder<RoadmapOutcomeData> = D.object({
  required: {
    id: D.number,
    title: D.string,
    body: D.string,
    horizon: D.nullable(D.literalUnion('now', 'next', 'later')),
    horizon_order: D.nullable(D.number),
    outcome_readiness: D.nullable(outcomeReadinessWire),
    iterations: D.array(roadmapIteration)
  }
})

const outcomeWhy = D.object({
  optional: {
    benefit: D.string,
    metric: D.string,
    confidence: D.string,
    value: D.string
  }
})

const roadmapCard: D.Decoder<RoadmapCard> = D.object({
  required: {
    id: D.number,
    name: D.string,
    why: outcomeWhy,
    horizon: D.literalUnion('now', 'next', 'later'),
    state: D.literalUnion('done', 'ready', 'needs-attention', 'planned'),
    gap_count: D.number,
    tone: D.literalUnion('now', 'ready', 'gap', 'future')
  }
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
      blockers: D.array(D.object({ required: { id: D.number, title: D.string, state: D.string } }))
    }
  }),
  D.object({ required: { domain: D.literal('board_invalidated'), reason: D.string } }),
  D.object({ required: { domain: D.literal('stories'), stories: D.array(story) } }),
  D.object({ required: { domain: D.literal('epics'), epics: D.array(story) } }),
  D.object({ required: { domain: D.literal('outcomes'), outcomes: D.array(roadmapOutcome) } }),
  D.object({
    required: { domain: D.literal('iterations'), iterations: D.array(boardIteration) }
  }),
  D.object({ required: { domain: D.literal('roadmap'), roadmap: D.array(roadmapCard) } })
)
