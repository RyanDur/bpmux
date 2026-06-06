import { useCallback } from 'react'
import { describeHttpError } from '../api/http'
import { postStoryLabels } from '../api/labelsClient'
import { computeMemberDropMutation } from './iterationPosition'
import { eagerReorder, positionInParentIteration } from './iterationMembers'
import { findCurrentIteration } from './currentIteration'
import { parseParentIterationLabel } from './labelSchema'
import { columnFor, storyTypeFromLabels, type BoardResponse, type ColumnState } from './types'

export interface UseBoardReorderOptions {
  data: BoardResponse | null
  setData: React.Dispatch<React.SetStateAction<BoardResponse | null>>
  setReorderAnnounce: (msg: string) => void
  setError: (msg: string) => void
  usingMock: boolean
  apiBase: string | undefined
  authToken: string | undefined
}

export type HandleReorder = (state: ColumnState, moved: number, targetNumber: number) => void

export const useBoardReorder = ({
  data,
  setData,
  setReorderAnnounce,
  setError,
  usingMock,
  apiBase,
  authToken
}: UseBoardReorderOptions): HandleReorder => {
  return useCallback(
    (state, moved, targetNumber) => {
      const prevData = data
      const inGroupOf = (resp: BoardResponse): BoardResponse['stories'] =>
        resp.stories.filter(
          (s) =>
            columnFor(s.state, s.gh_state === 'closed', storyTypeFromLabels(s.labels)) === state
        )

      if (state === 'ready' && prevData) {
        const iteration = findCurrentIteration(prevData.stories)
        if (iteration === null) return
        const readyStories = inGroupOf(prevData)
        const targetMemberIndex = readyStories.findIndex((s) => s.number === targetNumber)
        if (targetMemberIndex === -1) return
        const draggedStory = readyStories.find((s) => s.number === moved)
        const draggedOldPosition = draggedStory
          ? positionInParentIteration(draggedStory, iteration.number)
          : null
        if (draggedOldPosition === null) return
        const targetMemberPositions = readyStories
          .map((s) => positionInParentIteration(s, iteration.number))
          .filter((p): p is number => p !== null)
        const mutation = computeMemberDropMutation({
          draggedIterationNumber: iteration.number,
          draggedOldPosition,
          targetIterationNumber: iteration.number,
          targetMemberPositions,
          targetMemberIndex,
          targetMembersIncludeDragged: true
        })
        if (mutation === null) return
        const added = parseParentIterationLabel(mutation.add[0])
        if (added === null) return

        setData((prev) => {
          if (!prev) return prev
          return {
            ...prev,
            stories: prev.stories.map((s) =>
              s.number === moved
                ? {
                    ...s,
                    iteration_membership: {
                      iteration_number: iteration.number,
                      position: added.position
                    }
                  }
                : s
            )
          }
        })

        const orderedAfter = readyStories
          .map((s) => ({
            number: s.number,
            position:
              s.number === moved
                ? added.position
                : (positionInParentIteration(s, iteration.number) ?? Number.MAX_SAFE_INTEGER)
          }))
          .sort((a, b) => a.position - b.position)
        const newIndex = orderedAfter.findIndex((s) => s.number === moved)
        if (newIndex !== -1) {
          setReorderAnnounce(`Moved to position ${newIndex + 1} of ${readyStories.length}`)
        }

        if (usingMock || apiBase === undefined) return
        void postStoryLabels({ apiBase, authToken }, moved, mutation).onFailure((error) => {
          setData(prevData)
          setError(`Reorder failed: ${describeHttpError(error)}`)
        }).value
        return
      }

      let newPosition = 0
      let groupSize = 0
      if (prevData) {
        const reordered = eagerReorder(inGroupOf(prevData), (s) => s.number, moved, targetNumber)
        const movedIdx = reordered.findIndex((s) => s.number === moved)
        if (movedIdx !== -1) {
          newPosition = movedIdx + 1
          groupSize = reordered.length
        }
      }

      setData((prev) => {
        if (!prev) return prev
        const inGroup = inGroupOf(prev)
        const reordered = eagerReorder(inGroup, (s) => s.number, moved, targetNumber)
        if (reordered === inGroup) return prev
        const others = prev.stories.filter(
          (s) =>
            columnFor(s.state, s.gh_state === 'closed', storyTypeFromLabels(s.labels)) !== state
        )
        return { ...prev, stories: [...others, ...reordered] }
      })

      if (groupSize > 0) {
        setReorderAnnounce(`Moved to position ${newPosition} of ${groupSize}`)
      }
    },
    [data, setData, setReorderAnnounce, setError, usingMock, apiBase, authToken]
  )
}
