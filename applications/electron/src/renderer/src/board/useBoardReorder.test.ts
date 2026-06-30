import { describe, expect, test, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardReorder } from './useBoardReorder'
import { SessionActionType } from '../transport/session/actions'
import { type Story } from './types'

const ITER = 9000

const story = (id: number): Story => ({
  id: id,
  title: `s${id}`,
  state: 'ready',
  labels: ['Feature', 'ready'],
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

const reorderWith = (
  stories: Story[],
  iterationStories: number[]
): {
  run: (moved: number, targetId: number) => void
  dispatch: ReturnType<typeof vi.fn>
  pending: ReturnType<typeof vi.fn>
} => {
  const dispatch = vi.fn()
  const pending = vi.fn()
  const { result } = renderHook(() =>
    useBoardReorder({
      data: stories,
      setData: vi.fn(),
      setReorderAnnounce: vi.fn(),
      dispatch,
      currentIterationId: ITER,
      currentIterationStories: iterationStories,
      setPendingReadyOrder: pending
    })
  )
  return {
    run: (moved, targetId) => result.current('ready', moved, targetId),
    dispatch,
    pending
  }
}

describe('useBoardReorder — ready reorder dispatches the parent-held ordered-list move', () => {
  test('drag DOWN onto the last row dispatches add_story_to_iteration at the new 1-based index', () => {
    const { run, dispatch } = reorderWith([story(701), story(702), story(703)], [701, 702, 703])
    run(701, 703)
    expect(dispatch).toHaveBeenCalledWith({
      type: SessionActionType.AddStoryToIterationRequested,
      iterationId: ITER,
      storyId: 701,
      position: 3
    })
  })

  test('drag UP onto the first row dispatches position 1', () => {
    const { run, dispatch } = reorderWith([story(701), story(702), story(703)], [701, 702, 703])
    run(703, 701)
    expect(dispatch).toHaveBeenCalledWith({
      type: SessionActionType.AddStoryToIterationRequested,
      iterationId: ITER,
      storyId: 703,
      position: 1
    })
  })

  test('optimistically reflects the new ordered id-list so the column updates immediately', () => {
    const { run, pending } = reorderWith([story(701), story(702), story(703)], [701, 702, 703])
    run(701, 703)
    expect(pending).toHaveBeenCalledWith([702, 703, 701])
  })

  test('a drop that does not change the iteration order dispatches nothing', () => {
    const { run, dispatch, pending } = reorderWith([story(701), story(702)], [701, 702])
    run(999, 701)
    expect(dispatch).not.toHaveBeenCalled()
    expect(pending).not.toHaveBeenCalled()
  })
})
