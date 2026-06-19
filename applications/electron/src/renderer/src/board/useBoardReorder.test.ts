import { describe, expect, test, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardReorder } from './useBoardReorder'
import { parentIterationLabel, parseParentIterationLabel } from './labelSchema'
import type { BoardResponse, BoardStory, IterationMembership } from './types'

const { postStoryLabelsMock } = vi.hoisted(() => ({
  postStoryLabelsMock: vi.fn(
    (_ctx: unknown, _storyNumber: number, _mutation: { add: string[]; remove: string[] }) => ({
      onFailure: () => ({ value: undefined })
    })
  )
}))

vi.mock('../api/labelsClient', () => ({
  postStoryLabels: postStoryLabelsMock
}))

const ITER = 9000

const iterationStory = (): BoardStory => ({
  number: ITER,
  title: 'Iteration Current',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration', 'iteration-order.100'],
  is_current: true,
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

const story = (number: number, position: number): BoardStory => ({
  number,
  title: `s${number}`,
  state: 'ready',
  labels: ['Feature', 'ready'],
  iteration_membership: { iteration_number: ITER, position },
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

const reorderWith = (
  stories: BoardStory[]
): {
  run: (moved: number, targetNumber: number) => void
  membershipOf: (n: number) => IterationMembership | null | undefined
} => {
  let current: BoardResponse | null = { stories, fetched_at: '2026-05-10T00:00:00Z' }
  const setData = (updater: React.SetStateAction<BoardResponse | null>): void => {
    current = typeof updater === 'function' ? updater(current) : updater
  }
  const { result } = renderHook(() =>
    useBoardReorder({
      data: current,
      setData,
      setReorderAnnounce: vi.fn(),
      setError: vi.fn(),
      setPendingReadyOrder: vi.fn(),
      clearPendingReadyOrder: vi.fn(),
      apiBase: 'http://localhost',
      authToken: 'tok'
    })
  )
  return {
    run: (moved, targetNumber) => result.current('ready', moved, targetNumber),
    membershipOf: (n) => current?.stories.find((s) => s.number === n)?.iteration_membership
  }
}

describe('#1524 useBoardReorder — ready-queue reorders on the shared canonical (parent-iteration position)', () => {
  beforeEach(() => {
    postStoryLabelsMock.mockClear()
  })

  test('drag DOWN onto the last row persists a parent-iteration position, not ready-order', () => {
    const { run } = reorderWith([
      iterationStory(),
      story(701, 100),
      story(702, 200),
      story(703, 300)
    ])
    run(701, 703)
    const [, movedNumber, mutation] = postStoryLabelsMock.mock.calls[0]
    expect(movedNumber).toBe(701)
    expect(mutation.remove).toEqual([parentIterationLabel(ITER, 100)])
    const added = parseParentIterationLabel(mutation.add[0])
    expect(added?.iterationNumber).toBe(ITER)
    expect(added?.position).toBeGreaterThan(300)
    expect(mutation.add[0].startsWith('ready-order.')).toBe(false)
  })

  test('drag UP onto the first row persists a parent-iteration position below the target', () => {
    const { run } = reorderWith([
      iterationStory(),
      story(701, 100),
      story(702, 200),
      story(703, 300)
    ])
    run(703, 701)
    const [, movedNumber, mutation] = postStoryLabelsMock.mock.calls[0]
    expect(movedNumber).toBe(703)
    expect(mutation.remove).toEqual([parentIterationLabel(ITER, 300)])
    const added = parseParentIterationLabel(mutation.add[0])
    expect(added?.position).toBeLessThan(100)
  })

  test('optimistically updates the moved story iteration_membership so both views re-sort in-session', () => {
    const { run, membershipOf } = reorderWith([
      iterationStory(),
      story(701, 100),
      story(702, 200),
      story(703, 300)
    ])
    run(701, 703)
    const moved = membershipOf(701)
    expect(moved?.iteration_number).toBe(ITER)
    expect(moved?.position).toBeGreaterThan(300)
  })

  test('DENSE positions (gap<2) re-space the WHOLE member set to clean distinct gaps — no colliding/duplicate position (the snap-back + non-determinism cure)', () => {
    const { run, membershipOf } = reorderWith([
      iterationStory(),
      story(701, 1),
      story(702, 2),
      story(703, 3)
    ])
    run(703, 702)
    const p701 = membershipOf(701)?.position
    const p702 = membershipOf(702)?.position
    const p703 = membershipOf(703)?.position
    expect(new Set([p701, p702, p703]).size).toBe(3)
    for (const p of [p701, p702, p703]) {
      expect(p).toBeGreaterThanOrEqual(100)
    }
    expect(p701).toBeLessThan(p703 ?? 0)
    expect(p703).toBeLessThan(p702 ?? 0)
  })
})
