import { describe, expect, test } from 'vitest'
import { epicIdsInIteration, findCurrentIteration, isMemberOfIteration } from './currentIteration'
import { iterationLabel } from './labelSchema'
import type { BoardIterationData } from './selectIterations'
import type { Story } from './types'
const iteration = (overrides: Partial<BoardIterationData>): BoardIterationData => ({
  id: 9000,
  title: 'Iteration Lyra',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories: [],
  ...overrides
})

const story = (overrides: Partial<Story>): Story => ({
  id: 9100,
  title: 'member',
  state: 'icebox',
  gh_state: 'open',
  labels: ['feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1540 findCurrentIteration — explicit designation, never auto-picks', () => {
  test('returns null when no open iterations', () => {
    const closed = iteration({ id: 1, complete: true, is_current: true })
    expect(findCurrentIteration([closed])).toBeNull()
    expect(findCurrentIteration([])).toBeNull()
  })

  test('returns null (honest-empty) when no iteration is designated — never auto-picks lowest-order', () => {
    const a = iteration({ id: 1, title: 'A', iteration_order: 100 })
    const b = iteration({ id: 2, title: 'B', iteration_order: 200 })
    expect(findCurrentIteration([a, b])).toBeNull()
  })

  test('returns the explicitly-designated iteration, not the lowest iteration-order', () => {
    const lowerOrder = iteration({ id: 1, title: 'A', iteration_order: 100 })
    const designated = iteration({
      id: 2,
      title: 'B',
      iteration_order: 200,
      is_current: true
    })
    expect(findCurrentIteration([lowerOrder, designated])?.id).toBe(2)
  })

  test('reordering the plan does not change which iteration is current', () => {
    const designated = iteration({
      id: 1,
      title: 'A',
      iteration_order: 200,
      is_current: true
    })
    const lowerOrder = iteration({ id: 2, title: 'B', iteration_order: 100 })
    expect(findCurrentIteration([designated, lowerOrder])?.id).toBe(1)
  })

  test('a closed iteration is never current even if marked (designation is cleared at close)', () => {
    const closedMarked = iteration({ id: 1, complete: true, is_current: true })
    const open = iteration({ id: 2, complete: false })
    expect(findCurrentIteration([closedMarked, open])).toBeNull()
  })
})

describe('#1453 isMemberOfIteration', () => {
  test('a story listed in the iterations stories matches', () => {
    const iter = iteration({
      id: 1464,
      title: 'Iteration Reticulum',
      stories: [9501]
    })
    const member = story({ id: 9501 })
    expect(isMemberOfIteration(member, iter)).toBe(true)
  })

  test('legacy iteration-{Name}.{N} label alone no longer matches (identity-only attribution)', () => {
    const iter = iteration({ id: 1491, title: 'Iteration Caelum' })
    const member = story({
      id: 9601,
      labels: ['feature', iterationLabel('Caelum', 100)]
    })
    expect(isMemberOfIteration(member, iter)).toBe(false)
  })

  test('different iteration does not match', () => {
    const iter = iteration({ id: 1464, title: 'Iteration Reticulum' })
    const memberOfOther = story({ id: 9701 })
    expect(isMemberOfIteration(memberOfOther, iter)).toBe(false)
  })

  test('story without any iteration label / membership returns false', () => {
    const iter = iteration({ id: 1464, title: 'Iteration Reticulum' })
    const orphan = story({ id: 9801 })
    expect(isMemberOfIteration(orphan, iter)).toBe(false)
  })
})

describe('#1453 epicIdsInIteration', () => {
  test('returns distinct sorted parent_epic numbers of iteration members', () => {
    const iter = iteration({
      id: 1464,
      title: 'Iteration Reticulum',
      stories: [9501, 9502, 9503]
    })
    const s1 = story({ id: 9501, parent_epic: 200 })
    const s2 = story({ id: 9502, parent_epic: 100 })
    const s3 = story({ id: 9503, parent_epic: 200 })
    expect(epicIdsInIteration([s1, s2, s3], iter)).toEqual([100, 200])
  })

  test('ignores stories not in iteration', () => {
    const iter = iteration({
      id: 1464,
      title: 'Iteration Reticulum',
      stories: [9501]
    })
    const inIter = story({ id: 9501, parent_epic: 200 })
    const otherIter = story({ id: 9601, parent_epic: 300 })
    expect(epicIdsInIteration([inIter, otherIter], iter)).toEqual([200])
  })

  test('ignores stories without parent_epic', () => {
    const iter = iteration({
      id: 1464,
      title: 'Iteration Reticulum',
      stories: [9501]
    })
    const noEpic = story({ id: 9501 })
    expect(epicIdsInIteration([noEpic], iter)).toEqual([])
  })
})
