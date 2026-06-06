import { parseIterationOrderPosition, parseUnscheduledPosition } from './labelSchema'
import { postStoryLabels } from '../api/labelsClient'
import { storyTypeFromLabels, type BoardStory } from './types'

export interface DragPayload {
  storyNumber: number
  iterationNumber: number
  oldPosition: number
}

export interface IterationDragPayload {
  iterationNumber: number
}

export interface PendingMemberPosition {
  iterationNumber: number
  position: number
}

export const positionInIterationOrder = (story: BoardStory): number | null => {
  let lowest: number | null = null
  for (const label of story.labels) {
    const n = parseIterationOrderPosition(label)
    if (n !== null && (lowest === null || n < lowest)) lowest = n
  }
  return lowest
}

export const iterationOrderLabelsOf = (story: BoardStory): string[] =>
  story.labels.filter((label) => parseIterationOrderPosition(label) !== null)

export const isIterationStory = (story: BoardStory): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

export const positionInParentIteration = (
  story: BoardStory,
  iterationNumber: number
): number | null => {
  const membership = story.iteration_membership
  if (membership && membership.iteration_number === iterationNumber) {
    return membership.position
  }
  return null
}

export const positionInUnscheduled = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    const n = parseUnscheduledPosition(label)
    if (n !== null) return n
  }
  return null
}

export const isLooseStory = (story: BoardStory): boolean => {
  if (isIterationStory(story)) return false
  if (story.state !== 'icebox') return false
  return story.iteration_membership === undefined || story.iteration_membership === null
}

export const eagerReorder = <T>(
  items: T[],
  findId: (t: T) => number,
  fromId: number | null,
  toId: number | null
): T[] => {
  if (fromId === null || toId === null) return items
  const fromIdx = items.findIndex((t) => findId(t) === fromId)
  const toIdx = items.findIndex((t) => findId(t) === toId)
  if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return items
  const next = [...items]
  const [moved] = next.splice(fromIdx, 1)
  next.splice(toIdx, 0, moved)
  return next
}

export const postLabelMutation = (
  storyNumber: number,
  addLabels: string[],
  removeLabels: string[]
): void => {
  if (typeof window === 'undefined') return
  const bridge = window.bpmux
  if (!bridge?.apiBase) return
  void postStoryLabels({ apiBase: bridge.apiBase, authToken: bridge.authToken }, storyNumber, {
    add: addLabels,
    remove: removeLabels
  }).value
}
