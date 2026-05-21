import {
  parseIterationLabel,
  parseIterationOrderPosition,
  parseParentIterationLabel,
  parseUnscheduledPosition
} from './labelSchema'
import { postStoryLabels } from '../api/labelsClient'
import { storyTypeFromLabels, type BoardStory } from './types'

export interface DragPayload {
  storyNumber: number
  themeName: string
  oldPosition: number
}

export interface IterationDragPayload {
  iterationNumber: number
  oldOrder: number | null
}

export interface PendingMemberPosition {
  theme: string
  position: number
}

export const positionInIterationOrder = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    const n = parseIterationOrderPosition(label)
    if (n !== null) return n
  }
  return null
}

export const isIterationStory = (story: BoardStory): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

export const positionInIteration = (story: BoardStory, themeName: string): number | null => {
  for (const label of story.labels) {
    const parsed = parseIterationLabel(label)
    if (parsed?.name === themeName) return parsed.position
  }
  return null
}

export const positionInParentIteration = (
  story: BoardStory,
  iterationNumber: number
): number | null => {
  for (const label of story.labels) {
    const parsed = parseParentIterationLabel(label)
    if (parsed?.iterationNumber === iterationNumber) return parsed.position
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
  const hasNamedIterationLabel = story.labels.some((l) => {
    const parsed = parseIterationLabel(l)
    return parsed !== null && parsed.name !== 'Unscheduled'
  })
  return !hasNamedIterationLabel
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
