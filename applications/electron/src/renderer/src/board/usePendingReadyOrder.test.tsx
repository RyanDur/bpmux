import { describe, expect, test } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { usePendingReadyOrder } from './usePendingReadyOrder'
import { useBoardData } from './useBoardData'
import { asStoryId, type Story, type ColumnState } from './types'

const readyStory = (id: number, position: number): Story => ({
  id: asStoryId(id),
  title: `story-${id}`,
  state: 'ready',
  labels: ['ready', `parent-iteration-#1655.${position}`],
  assignees: [],
  created_at: '2026-06-07T00:00:00Z',
  body: '',
  iteration_membership: { iteration_number: 1655, position }
})

const board = (stories: Story[]): Story[] => stories

const readyOrder = (grouped: Map<ColumnState, Story[]>): number[] =>
  (grouped.get('ready') ?? []).map((s) => s.id)

describe('#1524 ready-queue optimistic reorder survives a stale re-fetch', () => {
  test('a drop holds its position when a board re-fetch still shows the old position', () => {
    const initial = board([readyStory(1686, 100), readyStory(1518, 112), readyStory(1687, 125)])
    const { result, rerender } = renderHook(
      ({ data }: { data: Story[] }) => {
        const { setPendingReadyOrder, effectiveReadyPosition } = usePendingReadyOrder(data)
        const boardData = useBoardData(data, [], effectiveReadyPosition)
        return { setPendingReadyOrder, grouped: boardData.grouped }
      },
      { initialProps: { data: initial } }
    )

    expect(readyOrder(result.current.grouped)).toEqual([1686, 1518, 1687])

    act(() => {
      result.current.setPendingReadyOrder(1687, 106)
    })

    const stale = board([readyStory(1686, 100), readyStory(1518, 112), readyStory(1687, 125)])
    rerender({ data: stale })

    expect(readyOrder(result.current.grouped)).toEqual([1686, 1687, 1518])
  })

  test('the hold releases once the re-fetch reflects the dropped position', () => {
    const initial = board([readyStory(1686, 100), readyStory(1518, 112), readyStory(1687, 125)])
    const { result, rerender } = renderHook(
      ({ data }: { data: Story[] }) => {
        const { setPendingReadyOrder, effectiveReadyPosition } = usePendingReadyOrder(data)
        const boardData = useBoardData(data, [], effectiveReadyPosition)
        return { setPendingReadyOrder, grouped: boardData.grouped }
      },
      { initialProps: { data: initial } }
    )

    act(() => {
      result.current.setPendingReadyOrder(1687, 106)
    })

    const settled = board([readyStory(1686, 100), readyStory(1687, 106), readyStory(1518, 112)])
    rerender({ data: settled })

    expect(readyOrder(result.current.grouped)).toEqual([1686, 1687, 1518])
  })

  test('clearing the hold lets a failed reorder revert to the server position', () => {
    const initial = board([readyStory(1686, 100), readyStory(1518, 112), readyStory(1687, 125)])
    const { result, rerender } = renderHook(
      ({ data }: { data: Story[] }) => {
        const { setPendingReadyOrder, clearPendingReadyOrder, effectiveReadyPosition } =
          usePendingReadyOrder(data)
        const boardData = useBoardData(data, [], effectiveReadyPosition)
        return { setPendingReadyOrder, clearPendingReadyOrder, grouped: boardData.grouped }
      },
      { initialProps: { data: initial } }
    )

    act(() => {
      result.current.setPendingReadyOrder(1687, 106)
    })
    expect(readyOrder(result.current.grouped)).toEqual([1686, 1687, 1518])

    act(() => {
      result.current.clearPendingReadyOrder([1687])
    })
    rerender({ data: initial })

    expect(readyOrder(result.current.grouped)).toEqual([1686, 1518, 1687])
  })
})
