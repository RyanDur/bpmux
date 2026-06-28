import { describe, expect, test } from 'vitest'
import {
  epicNumbersInIteration,
  findCurrentIteration,
  isMemberOfIteration
} from './currentIteration'
import { iterationLabel } from './labelSchema'
import type { BoardIterationData } from './selectIterations'
import type { BoardStory } from './types'

const iteration = (overrides: Partial<BoardIterationData>): BoardIterationData => ({
  number: 9000,
  title: 'Iteration Lyra',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  ...overrides
})

const story = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9100,
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
    const closed = iteration({ number: 1, complete: true, is_current: true })
    expect(findCurrentIteration([closed])).toBeNull()
    expect(findCurrentIteration([])).toBeNull()
  })

  test('returns null (honest-empty) when no iteration is designated — never auto-picks lowest-order', () => {
    const a = iteration({ number: 1, title: 'A', iteration_order: 100 })
    const b = iteration({ number: 2, title: 'B', iteration_order: 200 })
    expect(findCurrentIteration([a, b])).toBeNull()
  })

  test('returns the explicitly-designated iteration, not the lowest iteration-order', () => {
    const lowerOrder = iteration({ number: 1, title: 'A', iteration_order: 100 })
    const designated = iteration({ number: 2, title: 'B', iteration_order: 200, is_current: true })
    expect(findCurrentIteration([lowerOrder, designated])?.number).toBe(2)
  })

  test('reordering the plan does not change which iteration is current', () => {
    const designated = iteration({ number: 1, title: 'A', iteration_order: 200, is_current: true })
    const lowerOrder = iteration({ number: 2, title: 'B', iteration_order: 100 })
    expect(findCurrentIteration([designated, lowerOrder])?.number).toBe(1)
  })

  test('a closed iteration is never current even if marked (designation is cleared at close)', () => {
    const closedMarked = iteration({ number: 1, complete: true, is_current: true })
    const open = iteration({ number: 2, complete: false })
    expect(findCurrentIteration([closedMarked, open])).toBeNull()
  })
})

describe('#1453 isMemberOfIteration', () => {
  test('canonical iteration_membership matches', () => {
    const iter = iteration({ number: 1464, title: 'Iteration Reticulum' })
    const member = story({
      number: 9501,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    expect(isMemberOfIteration(member, iter)).toBe(true)
  })

  test('legacy iteration-{Name}.{N} label alone no longer matches (identity-only attribution)', () => {
    const iter = iteration({ number: 1491, title: 'Iteration Caelum' })
    const member = story({
      number: 9601,
      labels: ['feature', iterationLabel('Caelum', 100)]
    })
    expect(isMemberOfIteration(member, iter)).toBe(false)
  })

  test('different iteration does not match', () => {
    const iter = iteration({ number: 1464, title: 'Iteration Reticulum' })
    const memberOfOther = story({
      number: 9701,
      iteration_membership: { iteration_number: 1491, position: 100 }
    })
    expect(isMemberOfIteration(memberOfOther, iter)).toBe(false)
  })

  test('story without any iteration label / membership returns false', () => {
    const iter = iteration({ number: 1464, title: 'Iteration Reticulum' })
    const orphan = story({ number: 9801 })
    expect(isMemberOfIteration(orphan, iter)).toBe(false)
  })
})

describe('#1453 epicNumbersInIteration', () => {
  test('returns distinct sorted parent_epic numbers of iteration members', () => {
    const iter = iteration({ number: 1464, title: 'Iteration Reticulum' })
    const s1 = story({
      number: 9501,
      parent_epic: 200,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    const s2 = story({
      number: 9502,
      parent_epic: 100,
      iteration_membership: { iteration_number: 1464, position: 200 }
    })
    const s3 = story({
      number: 9503,
      parent_epic: 200,
      iteration_membership: { iteration_number: 1464, position: 300 }
    })
    expect(epicNumbersInIteration([s1, s2, s3], iter)).toEqual([100, 200])
  })

  test('ignores stories not in iteration', () => {
    const iter = iteration({ number: 1464, title: 'Iteration Reticulum' })
    const inIter = story({
      number: 9501,
      parent_epic: 200,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    const otherIter = story({
      number: 9601,
      parent_epic: 300,
      iteration_membership: { iteration_number: 1491, position: 100 }
    })
    expect(epicNumbersInIteration([inIter, otherIter], iter)).toEqual([200])
  })

  test('ignores stories without parent_epic', () => {
    const iter = iteration({ number: 1464, title: 'Iteration Reticulum' })
    const noEpic = story({
      number: 9501,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    expect(epicNumbersInIteration([noEpic], iter)).toEqual([])
  })
})
