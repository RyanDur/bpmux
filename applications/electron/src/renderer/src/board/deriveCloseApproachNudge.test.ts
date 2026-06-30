import { describe, expect, test } from 'vitest'
import { deriveCloseApproachNudge } from './deriveCloseApproachNudge'
import type { BoardIterationData } from './selectIterations'
import type { Story, IterationReadiness, StoryState } from './types'
let seq = 1
const iteration = (over: Partial<BoardIterationData>): BoardIterationData => ({
  id: seq++,
  title: 'Iteration X',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories: [],
  ...over
})

const member = (
  iterationNumber: number,
  position: number,
  state: StoryState,
  over: Partial<Story> = {}
): Story => ({
  id: seq++,
  title: 'a member',
  state,
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  gh_state: 'open',
  iteration_membership: { iteration_id: iterationNumber, position },
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

const drainingMembers = (iterationNumber: number): Story[] => [
  member(iterationNumber, 1, 'delivered'),
  member(iterationNumber, 2, 'delivered'),
  member(iterationNumber, 3, 'delivered'),
  member(iterationNumber, 4, 'in-progress')
]

describe('#1705 deriveCloseApproachNudge — warn when the NEXT iteration is not ready at close-approach', () => {
  test('FIRES: current draining (>=75% work-done, delivered pre-batch-accept) + next exists + next not-ready', () => {
    const members = drainingMembers(100)
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    const next = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    const nudge = deriveCloseApproachNudge([current, next], members)
    expect(nudge).not.toBeNull()
    expect(nudge?.nextIterationNumber).toBe(200)
    expect(nudge?.nextIterationName).toBe('Fornax')
    expect(nudge?.activeGapLabel).toBe('designer-review')
  })

  test('FIRES post-batch too: accepted members count as work-done (signal stable across the batch-accept)', () => {
    const members = [
      member(100, 1, 'accepted'),
      member(100, 2, 'accepted'),
      member(100, 3, 'accepted'),
      member(100, 4, 'in-progress')
    ]
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    const next = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    expect(deriveCloseApproachNudge([current, next], members)).not.toBeNull()
  })

  test('NULL: no current iteration (the #1684 no-iteration board state)', () => {
    const planned = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    expect(deriveCloseApproachNudge([planned], [])).toBeNull()
  })

  test('NULL: current not draining (<75% accepted)', () => {
    const members = [
      member(100, 1, 'accepted'),
      member(100, 2, 'in-progress'),
      member(100, 3, 'in-progress'),
      member(100, 4, 'ready')
    ]
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    const next = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    expect(deriveCloseApproachNudge([current, next], members)).toBeNull()
  })

  test('NULL: no next iteration exists (current is the last in order)', () => {
    const members = drainingMembers(100)
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    expect(deriveCloseApproachNudge([current], members)).toBeNull()
  })

  test('NULL: next iteration is ready (no alarm)', () => {
    const members = drainingMembers(100)
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    const next = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: READY
    })
    expect(deriveCloseApproachNudge([current, next], members)).toBeNull()
  })

  test('NULL: far-horizon — the NEXT (+1) is ready even though a +2 is not-ready (gates on +1, not any-future)', () => {
    const members = drainingMembers(100)
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    const next = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: READY
    })
    const farHorizon = iteration({
      id: 300,
      title: 'Iteration Hydra',
      iteration_order: 3,
      readiness: NOT_READY
    })
    expect(deriveCloseApproachNudge([current, next, farHorizon], members)).toBeNull()
  })

  test('retired members are excluded from the drain denominator', () => {
    const members = [
      member(100, 1, 'accepted'),
      member(100, 2, 'accepted'),
      member(100, 3, 'accepted'),
      member(100, 4, 'in-progress'),
      member(100, 5, 'icebox', { gh_state: 'closed' })
    ]
    const current = iteration({
      id: 100,
      title: 'Iteration Lupus',
      is_current: true,
      iteration_order: 1,
      stories: members.map((m) => m.id)
    })
    const next = iteration({
      id: 200,
      title: 'Iteration Fornax',
      iteration_order: 2,
      readiness: NOT_READY
    })
    const nudge = deriveCloseApproachNudge([current, next], members)
    expect(
      nudge,
      'retired (closed, not accepted) excluded → 3/4 accepted = 0.75, fires'
    ).not.toBeNull()
  })
})
