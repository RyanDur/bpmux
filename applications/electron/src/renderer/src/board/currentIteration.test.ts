import { describe, expect, test } from 'vitest'
import {
  epicNumbersInIteration,
  findCurrentIteration,
  isMemberOfIteration
} from './currentIteration'
import { iterationLabel } from './labelSchema'
import type { BoardStory } from './types'

const iterationStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9000,
  title: 'Iteration Lyra',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
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
    const closed = iterationStory({ number: 1, gh_state: 'closed', is_current: true })
    expect(findCurrentIteration([closed])).toBeNull()
    expect(findCurrentIteration([])).toBeNull()
  })

  test('returns null (honest-empty) when no iteration is designated — never auto-picks lowest-order', () => {
    const a = iterationStory({
      number: 1,
      title: 'A',
      labels: ['iteration', 'iteration-order.100']
    })
    const b = iterationStory({
      number: 2,
      title: 'B',
      labels: ['iteration', 'iteration-order.200']
    })
    expect(findCurrentIteration([a, b])).toBeNull()
  })

  test('returns the explicitly-designated iteration, not the lowest iteration-order', () => {
    const lowerOrder = iterationStory({
      number: 1,
      title: 'A',
      labels: ['iteration', 'iteration-order.100']
    })
    const designated = iterationStory({
      number: 2,
      title: 'B',
      labels: ['iteration', 'iteration-order.200'],
      is_current: true
    })
    expect(findCurrentIteration([lowerOrder, designated])?.number).toBe(2)
  })

  test('reordering the plan does not change which iteration is current', () => {
    const designated = iterationStory({
      number: 1,
      title: 'A',
      labels: ['iteration', 'iteration-order.200'],
      is_current: true
    })
    const lowerOrder = iterationStory({
      number: 2,
      title: 'B',
      labels: ['iteration', 'iteration-order.100']
    })
    expect(findCurrentIteration([designated, lowerOrder])?.number).toBe(1)
  })

  test('a closed iteration is never current even if marked (designation is cleared at close)', () => {
    const closedMarked = iterationStory({ number: 1, gh_state: 'closed', is_current: true })
    const open = iterationStory({ number: 2, gh_state: 'open' })
    expect(findCurrentIteration([closedMarked, open])).toBeNull()
  })

  test('in-flight member state is irrelevant — only the explicit designation determines current', () => {
    const a = iterationStory({
      number: 100,
      title: 'A',
      labels: ['iteration', 'iteration-order.100']
    })
    const b = iterationStory({
      number: 200,
      title: 'B',
      labels: ['iteration', 'iteration-order.200'],
      is_current: true
    })
    const aReady = story({
      number: 9001,
      state: 'ready',
      iteration_membership: { iteration_number: 100, position: 100 }
    })
    expect(findCurrentIteration([a, b, aReady])?.number).toBe(200)
  })
})

describe('#1453 isMemberOfIteration', () => {
  test('canonical iteration_membership matches', () => {
    const iter = iterationStory({ number: 1464, title: 'Iteration Reticulum' })
    const member = story({
      number: 9501,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    expect(isMemberOfIteration(member, iter)).toBe(true)
  })

  test('legacy iteration-{Name}.{N} label alone no longer matches (identity-only attribution)', () => {
    const iter = iterationStory({ number: 1491, title: 'Iteration Caelum' })
    const member = story({
      number: 9601,
      labels: ['feature', iterationLabel('Caelum', 100)]
    })
    expect(isMemberOfIteration(member, iter)).toBe(false)
  })

  test('different iteration does not match', () => {
    const iter = iterationStory({ number: 1464, title: 'Iteration Reticulum' })
    const memberOfOther = story({
      number: 9701,
      iteration_membership: { iteration_number: 1491, position: 100 }
    })
    expect(isMemberOfIteration(memberOfOther, iter)).toBe(false)
  })

  test('story without any iteration label / membership returns false', () => {
    const iter = iterationStory({ number: 1464, title: 'Iteration Reticulum' })
    const orphan = story({ number: 9801 })
    expect(isMemberOfIteration(orphan, iter)).toBe(false)
  })
})

describe('#1453 epicNumbersInIteration', () => {
  test('returns distinct sorted parent_epic numbers of iteration members', () => {
    const iter = iterationStory({ number: 1464, title: 'Iteration Reticulum' })
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
    const iter = iterationStory({ number: 1464, title: 'Iteration Reticulum' })
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
    const iter = iterationStory({ number: 1464, title: 'Iteration Reticulum' })
    const noEpic = story({
      number: 9501,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    expect(epicNumbersInIteration([noEpic], iter)).toEqual([])
  })
})
