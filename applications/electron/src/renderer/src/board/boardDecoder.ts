import * as D from 'schemawax'

const iterationMembership = D.object({
  required: { iteration_id: D.number, position: D.number }
})

const outcomeMembership = D.object({
  required: { outcome_id: D.number, position: D.number }
})

const outcomeReadiness = D.object({
  required: { iteration_count: D.number, gap_count: D.number, all_ready: D.boolean }
})

const iterationReadiness = D.object({
  required: {
    drafted: D.boolean,
    stories_ordered: D.boolean,
    review_closed: D.boolean,
    dev_ok: D.boolean
  }
})

const storyState = D.literalUnion(
  'icebox',
  'ready',
  'in-progress',
  'desk-check',
  'delivered',
  'ready-for-accept',
  'needs-fixes',
  'rejected',
  'accepted',
  'retired'
)

const acceptor = D.literalUnion('team', 'owner', 'dev')

const horizon = D.literalUnion('now', 'next', 'later')

export const story = D.object({
  required: {
    id: D.number,
    title: D.string,
    state: storyState,
    labels: D.array(D.string),
    assignees: D.array(D.string),
    created_at: D.string,
    body: D.string
  },
  optional: {
    gh_state: D.literalUnion('open', 'closed'),
    parent_epic: D.nullable(D.number),
    iteration_membership: D.nullable(iterationMembership),
    iteration_order: D.nullable(D.number),
    is_current: D.boolean,
    acceptor: D.nullable(acceptor),
    rework_comment: D.nullable(D.string),
    horizon: D.nullable(horizon),
    outcome_readiness: D.nullable(outcomeReadiness),
    outcome_membership: D.nullable(outcomeMembership),
    readiness: D.nullable(iterationReadiness)
  }
})
