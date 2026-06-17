import { describe, expect, test } from 'vitest'
import { deriveCloseApproachNudge } from './deriveCloseApproachNudge'
import type { BoardStory, IterationReadiness, StoryState } from './types'

let seq = 1
const iteration = (over: Partial<BoardStory>): BoardStory => ({
  number: seq++,
  title: 'Iteration X',
  state: 'in-progress',
  labels: ['iteration'],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  gh_state: 'open',
  ...over
})

const member = (
  iterationNumber: number,
  position: number,
  state: StoryState,
  over: Partial<BoardStory> = {}
): BoardStory => ({
  number: seq++,
  title: 'a member',
  state,
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  iteration_membership: { iteration_number: iterationNumber, position },
  ...over
})

const READY: IterationReadiness = {
  drafted: true,
  stories_ordered: true,
  review_closed: true,
  dev_ok: true
}
const NOT_READY: IterationReadiness = {
  drafted: true,
  stories_ordered: true,
  review_closed: false,
  dev_ok: false
}

const drainingMembers = (iterationNumber: number): BoardStory[] => [
  member(iterationNumber, 1, 'delivered'),
  member(iterationNumber, 2, 'delivered'),
  member(iterationNumber, 3, 'delivered'),
  member(iterationNumber, 4, 'in-progress')
]

describe('#1705 deriveCloseApproachNudge — warn when the NEXT iteration is not ready at close-approach', () => {
  test('FIRES: current draining (>=75% work-done, delivered pre-batch-accept) + next exists + next not-ready', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    const next = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    const nudge = deriveCloseApproachNudge([current, next, ...drainingMembers(100)])
    expect(nudge).not.toBeNull()
    expect(nudge?.nextIterationNumber).toBe(200)
    expect(nudge?.nextIterationName).toBe('Fornax')
    expect(nudge?.activeGapLabel).toBe('designer-review')
  })

  test('FIRES post-batch too: accepted members count as work-done (signal stable across the batch-accept)', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    const next = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    const members = [
      member(100, 1, 'accepted'),
      member(100, 2, 'accepted'),
      member(100, 3, 'accepted'),
      member(100, 4, 'in-progress')
    ]
    expect(deriveCloseApproachNudge([current, next, ...members])).not.toBeNull()
  })

  test('NULL: no current iteration (the #1684 no-iteration board state)', () => {
    const planned = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    expect(deriveCloseApproachNudge([planned])).toBeNull()
  })

  test('NULL: current not draining (<75% accepted)', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    const next = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    const members = [
      member(100, 1, 'accepted'),
      member(100, 2, 'in-progress'),
      member(100, 3, 'in-progress'),
      member(100, 4, 'ready')
    ]
    expect(deriveCloseApproachNudge([current, next, ...members])).toBeNull()
  })

  test('NULL: no next iteration exists (current is the last in order)', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    expect(deriveCloseApproachNudge([current, ...drainingMembers(100)])).toBeNull()
  })

  test('NULL: next iteration is ready (no alarm)', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    const next = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: READY
    })
    expect(deriveCloseApproachNudge([current, next, ...drainingMembers(100)])).toBeNull()
  })

  test('NULL: far-horizon — the NEXT (+1) is ready even though a +2 is not-ready (gates on +1, not any-future)', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    const next = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: READY
    })
    const farHorizon = iteration({
      number: 300,
      title: 'Iteration Hydra',
      iteration_order: 3,
      readiness: NOT_READY
    })
    expect(
      deriveCloseApproachNudge([current, next, farHorizon, ...drainingMembers(100)])
    ).toBeNull()
  })

  test('retired members are excluded from the drain denominator', () => {
    const current = iteration({
      number: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1
    })
    const next = iteration({
      number: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    const members = [
      member(100, 1, 'accepted'),
      member(100, 2, 'accepted'),
      member(100, 3, 'accepted'),
      member(100, 4, 'in-progress'),
      member(100, 5, 'icebox', { gh_state: 'closed' })
    ]
    const nudge = deriveCloseApproachNudge([current, next, ...members])
    expect(
      nudge,
      'retired (closed, not accepted) excluded → 3/4 accepted = 0.75, fires'
    ).not.toBeNull()
  })
})
