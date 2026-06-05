import { useCallback } from 'react'
import { describeHttpError } from '../api/http'
import { postStoryLabels } from '../api/labelsClient'
import { computeReadyDropMutation } from './iterationPosition'
import { eagerReorder } from './iterationMembers'
import { parseReadyOrderPosition } from './labelSchema'
import { columnFor, storyTypeFromLabels, type BoardResponse, type ColumnState } from './types'

const readyOrderFromLabels = (labels: string[]): number | null => {
  for (const l of labels) {
    const n = parseReadyOrderPosition(l)
    if (n !== null) return n
  }
  return null
}

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

      if (usingMock || apiBase === undefined) return
      if (state === 'ready' && prevData) {
        const readyStories = inGroupOf(prevData)
        const targetMemberIndex = readyStories.findIndex((s) => s.number === targetNumber)
        if (targetMemberIndex === -1) return
        const targetMemberPositions = readyStories
          .map((s) => readyOrderFromLabels(s.labels))
          .filter((p): p is number => p !== null)
        const draggedStory = readyStories.find((s) => s.number === moved)
        const draggedOldPosition = draggedStory ? readyOrderFromLabels(draggedStory.labels) : null
        const mutation = computeReadyDropMutation({
          draggedOldPosition,
          targetMemberPositions,
          targetMemberIndex,
          targetMembersIncludeDragged: draggedOldPosition !== null
        })
        if (mutation === null) return
        void postStoryLabels({ apiBase, authToken }, moved, mutation).onFailure((error) => {
          setData(prevData)
          setError(`Reorder failed: ${describeHttpError(error)}`)
        }).value
      }
    },
    [data, setData, setReorderAnnounce, setError, usingMock, apiBase, authToken]
  )
}
