import { useCallback } from 'react'
import { describeHttpError } from '../api/http'
import { postStoryLabels } from '../api/labelsClient'
import {
  computeMemberDropMutation,
  computeMemberRespace,
  insertPositionWouldCascade,
  type DropMutation
} from './iterationPosition'
import { eagerReorder } from './iterationMembers'
import { parseParentIterationLabel } from './labelSchema'
import { columnFor, storyTypeFromLabels, type BoardResponse, type ColumnState } from './types'

export interface UseBoardReorderOptions {
  data: BoardResponse | null
  setData: React.Dispatch<React.SetStateAction<BoardResponse | null>>
  setReorderAnnounce: (msg: string) => void
  setError: (msg: string) => void
  setPendingReadyOrder: (storyNumber: number, position: number) => void
  clearPendingReadyOrder: (storyNumbers: number[]) => void
  apiBase: string | undefined
  authToken: string | undefined
}

export type HandleReorder = (state: ColumnState, moved: number, targetNumber: number) => void

export const useBoardReorder = ({
  data,
  setData,
  setReorderAnnounce,
  setError,
  setPendingReadyOrder,
  clearPendingReadyOrder,
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
        const readyStories = inGroupOf(prevData)
        const draggedStory = readyStories.find((s) => s.number === moved)
        const membership = draggedStory?.iteration_membership
        if (membership === undefined || membership === null) return
        const iterationNumber = membership.iteration_number
        const draggedOldPosition = membership.position
        const targetMemberIndex = readyStories.findIndex((s) => s.number === targetNumber)
        if (targetMemberIndex === -1) return
        const targetMemberPositions = readyStories
          .map((s) => s.iteration_membership?.position)
          .filter((p): p is number => p !== undefined && p !== null)

        const assignments: Array<{
          storyNumber: number
          position: number
          mutation: DropMutation
        }> = []
        if (
          insertPositionWouldCascade(draggedOldPosition, targetMemberPositions, targetMemberIndex)
        ) {
          const newSequence = eagerReorder(readyStories, (s) => s.number, moved, targetNumber)
          const respaced = computeMemberRespace(
            iterationNumber,
            newSequence.map((s) => ({
              storyNumber: s.number,
              currentPosition: s.iteration_membership?.position ?? null
            }))
          )
          for (const r of respaced) {
            assignments.push({
              storyNumber: r.storyNumber,
              position: r.position,
              mutation: { add: r.add, remove: r.remove }
            })
          }
        } else {
          const mutation = computeMemberDropMutation({
            draggedIterationNumber: iterationNumber,
            draggedOldPosition,
            targetIterationNumber: iterationNumber,
            targetMemberPositions,
            targetMemberIndex,
            targetMembersIncludeDragged: true
          })
          if (mutation === null) return
          const added = parseParentIterationLabel(mutation.add[0])
          if (added === null) return
          assignments.push({ storyNumber: moved, position: added.position, mutation })
        }
        if (assignments.length === 0) return

        const positionByStory = new Map(assignments.map((a) => [a.storyNumber, a.position]))
        for (const assignment of assignments) {
          setPendingReadyOrder(assignment.storyNumber, assignment.position)
        }
        setData((prev) => {
          if (!prev) return prev
          return {
            ...prev,
            stories: prev.stories.map((s) => {
              const newPosition = positionByStory.get(s.number)
              return newPosition === undefined
                ? s
                : {
                    ...s,
                    iteration_membership: {
                      iteration_number: iterationNumber,
                      position: newPosition
                    }
                  }
            })
          }
        })

        const orderedAfter = readyStories
          .map((s) => ({
            number: s.number,
            position:
              positionByStory.get(s.number) ??
              s.iteration_membership?.position ??
              Number.MAX_SAFE_INTEGER
          }))
          .sort((a, b) => a.position - b.position)
        const newIndex = orderedAfter.findIndex((s) => s.number === moved)
        if (newIndex !== -1) {
          setReorderAnnounce(`Moved to position ${newIndex + 1} of ${readyStories.length}`)
        }

        if (apiBase === undefined) return
        for (const assignment of assignments) {
          void postStoryLabels(
            { apiBase, authToken },
            assignment.storyNumber,
            assignment.mutation
          ).onFailure((error) => {
            setData(prevData)
            clearPendingReadyOrder(assignments.map((a) => a.storyNumber))
            setError(`Reorder failed: ${describeHttpError(error)}`)
          }).value
        }
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
    [
      data,
      setData,
      setReorderAnnounce,
      setError,
      setPendingReadyOrder,
      clearPendingReadyOrder,
      apiBase,
      authToken
    ]
  )
}
