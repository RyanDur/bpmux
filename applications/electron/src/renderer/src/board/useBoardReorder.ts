import { useCallback } from 'react'
import { describeHttpError } from '../api/http'
import { postStoryLabels } from '../api/labelsClient'
import { computeReadyDropMutation } from './iterationPosition'
import { parseReadyOrderPosition } from './labelSchema'
import {
  columnFor,
  storyTypeFromLabels,
  type BoardResponse,
  type ColumnState
} from './types'

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

export type HandleReorder = (
  state: ColumnState,
  moved: number,
  insertAt: number
) => void

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
    (state, moved, insertAt) => {
      const prevData = data
      let newPosition = 0
      let groupSize = 0
      if (prevData) {
        const inGroup = prevData.stories.filter(
          (s) =>
            columnFor(s.state, s.gh_state === 'closed', storyTypeFromLabels(s.labels)) ===
            state
        )
        const fromIdx = inGroup.findIndex((s) => s.number === moved)
        if (fromIdx !== -1) {
          const clampedInsert = Math.max(
            0,
            Math.min(insertAt - (insertAt > fromIdx ? 1 : 0), inGroup.length - 1)
          )
          newPosition = clampedInsert + 1
          groupSize = inGroup.length
        }
      }
      setData((prev) => {
        if (!prev) return prev
        const inGroup = prev.stories.filter(
          (s) =>
            columnFor(s.state, s.gh_state === 'closed', storyTypeFromLabels(s.labels)) ===
            state
        )
        const others = prev.stories.filter(
          (s) =>
            columnFor(s.state, s.gh_state === 'closed', storyTypeFromLabels(s.labels)) !==
            state
        )
        const fromIdx = inGroup.findIndex((s) => s.number === moved)
        if (fromIdx === -1) return prev
        const [movedStory] = inGroup.splice(fromIdx, 1)
        const clampedInsert = Math.max(
          0,
          Math.min(insertAt - (insertAt > fromIdx ? 1 : 0), inGroup.length)
        )
        inGroup.splice(clampedInsert, 0, movedStory)
        return { ...prev, stories: [...others, ...inGroup] }
      })
      if (groupSize > 0) {
        setReorderAnnounce(`Moved to position ${newPosition} of ${groupSize}`)
      }
      if (usingMock || apiBase === undefined) return
      if (state === 'ready' && prevData) {
        const readyStories = prevData.stories.filter(
          (s) =>
            columnFor(s.state, s.gh_state === 'closed', storyTypeFromLabels(s.labels)) ===
            'ready'
        )
        const targetMemberPositions = readyStories
          .map((s) => readyOrderFromLabels(s.labels))
          .filter((p): p is number => p !== null)
        const draggedStory = readyStories.find((s) => s.number === moved)
        const draggedOldPosition = draggedStory
          ? readyOrderFromLabels(draggedStory.labels)
          : null
        const mutation = computeReadyDropMutation({
          draggedOldPosition,
          targetMemberPositions,
          targetMemberIndex: Math.max(0, insertAt),
          targetMembersIncludeDragged: draggedOldPosition !== null
        })
        if (mutation === null) return
        void postStoryLabels({ apiBase, authToken }, moved, mutation)
          .onFailure((error) => {
            setData(prevData)
            setError(`Reorder failed: ${describeHttpError(error)}`)
          }).value
      }
    },
    [data, setData, setReorderAnnounce, setError, usingMock, apiBase, authToken]
  )
}
