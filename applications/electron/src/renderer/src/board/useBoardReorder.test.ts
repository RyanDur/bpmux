import { describe, expect, test, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardReorder } from './useBoardReorder'
import { readyOrderLabel } from './labelSchema'
import type { BoardResponse, BoardStory } from './types'

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

const story = (number: number, position: number): BoardStory => ({
  number,
  title: `s${number}`,
  state: 'ready',
  labels: ['Feature', 'ready', readyOrderLabel(position)],
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

const reorderWith = (
  stories: BoardStory[]
): {
  run: (moved: number, targetNumber: number) => void
  order: () => number[]
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
      usingMock: false,
      apiBase: 'http://localhost',
      authToken: 'tok'
    })
  )
  return {
    run: (moved, targetNumber) => result.current('ready', moved, targetNumber),
    order: () => (current?.stories ?? []).map((s) => s.number)
  }
}

describe('#1524 useBoardReorder — optimistic order and persisted position agree (no off-by-one)', () => {
  beforeEach(() => {
    postStoryLabelsMock.mockClear()
  })

  test('drag DOWN onto the last row: eager order == persisted position', () => {
    const { run, order } = reorderWith([story(701, 100), story(702, 200), story(703, 300)])
    run(701, 703)
    expect(order()).toEqual([702, 703, 701])
    const [, movedNumber, mutation] = postStoryLabelsMock.mock.calls[0]
    expect(movedNumber).toBe(701)
    expect(mutation.remove).toEqual([readyOrderLabel(100)])
    const positionLandingAfter703 = Number(mutation.add[0].replace('ready-order.', ''))
    expect(positionLandingAfter703).toBeGreaterThan(300)
  })

  test('drag UP onto the first row: eager order == persisted position', () => {
    const { run, order } = reorderWith([story(701, 100), story(702, 200), story(703, 300)])
    run(703, 701)
    expect(order()).toEqual([703, 701, 702])
    const [, movedNumber, mutation] = postStoryLabelsMock.mock.calls[0]
    expect(movedNumber).toBe(703)
    expect(mutation.remove).toEqual([readyOrderLabel(300)])
    const positionLandingBefore701 = Number(mutation.add[0].replace('ready-order.', ''))
    expect(positionLandingBefore701).toBeLessThan(100)
  })
})
