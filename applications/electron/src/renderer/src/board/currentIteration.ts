import {
  isIterationStory,
  positionInIteration,
  positionInIterationOrder,
  positionInParentIteration
} from './iterationMembers'
import { themeNameFromTitle } from './labelSchema'
import type { BoardStory, StoryState } from './types'

const IN_FLIGHT_STORY_STATES: ReadonlySet<StoryState> = new Set([
  'ready',
  'in-progress',
  'desk-check',
  'delivered',
  'ready-for-accept',
  'needs-fixes',
  'rejected'
])

const compareByIterationOrder = (a: BoardStory, b: BoardStory): number => {
  const ao = positionInIterationOrder(a)
  const bo = positionInIterationOrder(b)
  if (ao !== null && bo !== null) return ao - bo
  if (ao !== null) return -1
  if (bo !== null) return 1
  return a.number - b.number
}

export const findCurrentIteration = (stories: BoardStory[]): BoardStory | null => {
  const openIterations = stories.filter((s) => isIterationStory(s) && s.gh_state !== 'closed')
  if (openIterations.length === 0) return null
  const inFlightIterations = openIterations.filter((iter) =>
    stories.some(
      (s) =>
        !isIterationStory(s) &&
        IN_FLIGHT_STORY_STATES.has(s.state) &&
        isMemberOfIteration(s, iter)
    )
  )
  const candidates = inFlightIterations.length > 0 ? inFlightIterations : openIterations
  const ordered = [...candidates].sort(compareByIterationOrder)
  return ordered[0] ?? null
}

export const isMemberOfIteration = (story: BoardStory, iteration: BoardStory): boolean => {
  if (positionInParentIteration(story, iteration.number) !== null) return true
  const themeName = themeNameFromTitle(iteration.title)
  return positionInIteration(story, themeName) !== null
}

export const epicNumbersInIteration = (stories: BoardStory[], iteration: BoardStory): number[] => {
  const epicSet = new Set<number>()
  for (const s of stories) {
    if (!isMemberOfIteration(s, iteration)) continue
    if (s.parent_epic !== undefined) epicSet.add(s.parent_epic)
  }
  return Array.from(epicSet).sort((a, b) => a - b)
}
