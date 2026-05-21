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

describe('#1453 findCurrentIteration', () => {
  test('returns null when no open iterations', () => {
    const closed = iterationStory({ number: 1, gh_state: 'closed' })
    expect(findCurrentIteration([closed])).toBeNull()
    expect(findCurrentIteration([])).toBeNull()
  })

  test('falls back to lowest iteration-order.N when no iter has in-flight children', () => {
    const b = iterationStory({
      number: 2,
      title: 'B',
      labels: ['iteration', 'iteration-order.200']
    })
    const a = iterationStory({
      number: 1,
      title: 'A',
      labels: ['iteration', 'iteration-order.100']
    })
    const c = iterationStory({
      number: 3,
      title: 'C',
      labels: ['iteration', 'iteration-order.300']
    })
    expect(findCurrentIteration([b, a, c])?.number).toBe(1)
  })

  test('falls back to lowest issue number when no iteration-order labels + no in-flight', () => {
    const a = iterationStory({ number: 5, title: 'A' })
    const b = iterationStory({ number: 3, title: 'B' })
    expect(findCurrentIteration([a, b])?.number).toBe(3)
  })

  test('excludes closed iterations from candidacy', () => {
    const closed = iterationStory({
      number: 1,
      gh_state: 'closed',
      labels: ['iteration', 'iteration-order.100']
    })
    const open = iterationStory({
      number: 2,
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.200']
    })
    expect(findCurrentIteration([closed, open])?.number).toBe(2)
  })

  test('#1453-rework: picks iter with in-flight children over iter with only-iceboxed children', () => {
    const microscopium = iterationStory({
      number: 1398,
      title: 'Iteration Microscopium',
      labels: ['iteration', 'iteration-order.100']
    })
    const pictor = iterationStory({
      number: 1458,
      title: 'Iteration Pictor',
      labels: ['iteration', 'iteration-order.200']
    })
    const microscopiumIcebox = story({
      number: 9001,
      state: 'icebox',
      iteration_membership: { iteration_number: 1398, position: 100 }
    })
    const pictorInProgress = story({
      number: 9002,
      state: 'in-progress',
      iteration_membership: { iteration_number: 1458, position: 100 }
    })
    expect(
      findCurrentIteration([microscopium, pictor, microscopiumIcebox, pictorInProgress])?.number
    ).toBe(1458)
  })

  test('#1453-rework: tie-breaker = lowest iteration-order among multiple in-flight iters', () => {
    const iterA = iterationStory({
      number: 100,
      title: 'A',
      labels: ['iteration', 'iteration-order.200']
    })
    const iterB = iterationStory({
      number: 200,
      title: 'B',
      labels: ['iteration', 'iteration-order.100']
    })
    const aMember = story({
      number: 9001,
      state: 'ready',
      iteration_membership: { iteration_number: 100, position: 100 }
    })
    const bMember = story({
      number: 9002,
      state: 'in-progress',
      iteration_membership: { iteration_number: 200, position: 100 }
    })
    expect(findCurrentIteration([iterA, iterB, aMember, bMember])?.number).toBe(200)
  })

  test('#1453-rework: in-flight states covered (ready / in-progress / desk-check / delivered / ready-for-accept / needs-fixes / rejected)', () => {
    const inFlightStates = [
      'ready',
      'in-progress',
      'desk-check',
      'delivered',
      'ready-for-accept',
      'needs-fixes',
      'rejected'
    ] as const
    for (const state of inFlightStates) {
      const candidate = iterationStory({
        number: 1,
        title: 'A',
        labels: ['iteration', 'iteration-order.200']
      })
      const other = iterationStory({
        number: 2,
        title: 'B',
        labels: ['iteration', 'iteration-order.100']
      })
      const inFlightChild = story({
        number: 9001,
        state,
        iteration_membership: { iteration_number: 1, position: 100 }
      })
      const otherIcebox = story({
        number: 9002,
        state: 'icebox',
        iteration_membership: { iteration_number: 2, position: 100 }
      })
      expect(findCurrentIteration([candidate, other, inFlightChild, otherIcebox])?.number).toBe(1)
    }
  })

  test('#1453-rework: terminal-state children (accepted / retired) do NOT promote iter as current', () => {
    const a = iterationStory({
      number: 1,
      title: 'A',
      labels: ['iteration', 'iteration-order.200']
    })
    const b = iterationStory({
      number: 2,
      title: 'B',
      labels: ['iteration', 'iteration-order.100']
    })
    const acceptedChildOfA = story({
      number: 9001,
      state: 'accepted',
      iteration_membership: { iteration_number: 1, position: 100 }
    })
    const retiredChildOfA = story({
      number: 9002,
      state: 'retired',
      iteration_membership: { iteration_number: 1, position: 200 }
    })
    expect(findCurrentIteration([a, b, acceptedChildOfA, retiredChildOfA])?.number).toBe(2)
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

  test('legacy iteration-{Theme}.{N} label matches', () => {
    const iter = iterationStory({ number: 1491, title: 'Iteration Caelum' })
    const member = story({
      number: 9601,
      labels: ['feature', iterationLabel('Caelum', 100)]
    })
    expect(isMemberOfIteration(member, iter)).toBe(true)
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
