import { describe, expect, test } from 'vitest'
import { positionInIterationOrder } from './iterationMembers'
import type { BoardStory } from './types'

const iteration = (number: number, order: number | null, labels: string[] = []): BoardStory => ({
  number,
  title: `Iteration ${number}`,
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration', ...labels],
  iteration_order: order,
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: ''
})

describe('positionInIterationOrder — single-source backend field', () => {
  test('reads the backend iteration_order field', () => {
    expect(positionInIterationOrder(iteration(1735, 200))).toBe(200)
  })

  test('returns null when the field is absent (no issue-number fallback)', () => {
    expect(positionInIterationOrder(iteration(1735, null))).toBeNull()
  })

  test('reads the field, never a stale iteration-order label (single-source)', () => {
    const story = iteration(1735, 100, ['iteration-order.999'])
    expect(positionInIterationOrder(story)).toBe(100)
  })
})
