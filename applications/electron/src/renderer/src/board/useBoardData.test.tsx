import { describe, expect, test } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardData } from './useBoardData'
import type { BoardIterationData } from './selectIterations'
import type { BoardResponse, BoardStory } from './types'

const story = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 0,
  title: 'story',
  state: 'icebox',
  labels: [],
  assignees: [],
  created_at: '2026-05-24T00:00:00Z',
  body: '',
  ...overrides
})

const board = (stories: BoardStory[]): BoardResponse => ({
  stories,
  fetched_at: '2026-05-24T00:00:00Z'
})

const iteration = (number: number, stories: number[]): BoardIterationData => ({
  number,
  title: `Iteration ${number}`,
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories
})

const containerNumbers = (data: BoardResponse, iterations: BoardIterationData[] = []): number[] => {
  const { result } = renderHook(() => useBoardData(data, iterations))
  return result.current.iterationContainerStories.map((s) => s.number)
}

describe('#1547 facet 1 — iterationContainerStories gates on membership, not workflow-state', () => {
  test('a canonical member promoted to ready stays in the iteration container (empty-Crux fix)', () => {
    const member = story({ number: 1547, state: 'ready', labels: ['ready', 'bug'] })
    expect(containerNumbers(board([member]), [iteration(1543, [1547])])).toContain(1547)
  })

  test('a canonical member is included in every active workflow-state', () => {
    const states = ['ready', 'in-progress', 'desk-check', 'delivered'] as const
    const members = states.map((state, i) =>
      story({ number: 2000 + i, state, labels: [state, 'feature'] })
    )
    const iter = iteration(
      1543,
      members.map((m) => m.number)
    )
    const got = containerNumbers(board(members), [iter])
    for (const m of members) expect(got).toContain(m.number)
  })

  test('a loose icebox story (no membership) still appears in the container', () => {
    const loose = story({ number: 900, state: 'icebox', labels: ['chore'] })
    expect(containerNumbers(board([loose]))).toContain(900)
  })

  test('a ready story that is neither an iteration nor a member is NOT in the container', () => {
    const looseReady = story({ number: 901, state: 'ready', labels: ['ready', 'chore'] })
    expect(containerNumbers(board([looseReady]))).not.toContain(901)
  })
})
