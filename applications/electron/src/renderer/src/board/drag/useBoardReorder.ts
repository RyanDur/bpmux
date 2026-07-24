import { useCallback } from 'react'
import { eagerReorder } from '../iterationMembers'
import { columnFor, type Story, type ColumnState } from '../types'
import { addStoryToIterationRequested, type StateAction } from '../../state/actions'

export type UseBoardReorderOptions = {
  data: Story[] | null
  setData: React.Dispatch<React.SetStateAction<Story[] | null>>
  setReorderAnnounce: (msg: string) => void
  dispatch: (action: StateAction) => void
  currentIterationId: number | null
  currentIterationStories: number[]
  setPendingReadyOrder: (order: number[]) => void
}

export type HandleReorder = (state: ColumnState, moved: number, targetId: number) => void

export const useBoardReorder = ({
  data,
  setData,
  setReorderAnnounce,
  dispatch,
  currentIterationId,
  currentIterationStories,
  setPendingReadyOrder
}: UseBoardReorderOptions): HandleReorder => {
  return useCallback(
    (state, moved, targetId) => {
      const prevData = data
      const inGroupOf = (resp: Story[]): Story[] => resp.filter((s) => columnFor(s.state) === state)

      if (state === 'ready' && prevData) {
        if (currentIterationId === null) return
        const newOrder = eagerReorder(currentIterationStories, (n) => n, moved, targetId)
        if (newOrder === currentIterationStories) return
        const position = newOrder.indexOf(moved) + 1
        setPendingReadyOrder(newOrder)
        const readyStories = inGroupOf(prevData)
        const newReadySequence = eagerReorder(readyStories, (s) => s.id, moved, targetId)
        const readyIndex = newReadySequence.findIndex((s) => s.id === moved)
        if (readyIndex !== -1) {
          setReorderAnnounce(`Moved to position ${readyIndex + 1} of ${readyStories.length}`)
        }
        dispatch(addStoryToIterationRequested(currentIterationId, moved, position))
        return
      }

      let newPosition = 0
      let groupSize = 0
      if (prevData) {
        const reordered = eagerReorder(inGroupOf(prevData), (s) => s.id, moved, targetId)
        const movedIdx = reordered.findIndex((s) => s.id === moved)
        if (movedIdx !== -1) {
          newPosition = movedIdx + 1
          groupSize = reordered.length
        }
      }

      setData((prev) => {
        if (!prev) return prev
        const inGroup = inGroupOf(prev)
        const reordered = eagerReorder(inGroup, (s) => s.id, moved, targetId)
        if (reordered === inGroup) return prev
        const others = prev.filter((s) => columnFor(s.state) !== state)
        return [...others, ...reordered]
      })

      if (groupSize > 0) {
        setReorderAnnounce(`Moved to position ${newPosition} of ${groupSize}`)
      }
    },
    [
      data,
      setData,
      setReorderAnnounce,
      dispatch,
      currentIterationId,
      currentIterationStories,
      setPendingReadyOrder
    ]
  )
}
