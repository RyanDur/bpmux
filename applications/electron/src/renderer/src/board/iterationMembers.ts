import { parseUnscheduledPosition } from './labelSchema'
import { storyTypeFromLabels, type Story, type IterationMembership } from './types'

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

export type MembershipIndex = Map<number, IterationMembership>

export const buildMembershipIndex = (
  iterations: { number: number; stories: number[] }[]
): MembershipIndex => {
  const index: MembershipIndex = new Map()
  for (const iteration of iterations) {
    iteration.stories.forEach((storyNumber, i) => {
      index.set(storyNumber, { iteration_number: iteration.number, position: i + 1 })
    })
  }
  return index
}

export const isIterationStory = (story: Story): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

export const positionInParentIteration = (
  index: MembershipIndex,
  storyNumber: number,
  iterationNumber: number
): number | null => {
  const membership = index.get(storyNumber)
  if (membership && membership.iteration_number === iterationNumber) {
    return membership.position
  }
  return null
}

export const positionInUnscheduled = (story: Story): number | null => {
  for (const label of story.labels) {
    const n = parseUnscheduledPosition(label)
    if (n !== null) return n
  }
  return null
}

export const isLooseStory = (story: Story, index: MembershipIndex): boolean => {
  if (isIterationStory(story)) return false
  if (story.state !== 'icebox') return false
  return !index.has(story.id)
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
