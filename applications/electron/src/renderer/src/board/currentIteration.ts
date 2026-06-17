import { isIterationStory, positionInParentIteration } from './iterationMembers'
import type { BoardStory } from './types'

export const findCurrentIteration = (stories: BoardStory[]): BoardStory | null => {
  return (
    stories.find((s) => isIterationStory(s) && s.gh_state !== 'closed' && s.is_current === true) ??
    null
  )
}

export const isMemberOfIteration = (story: BoardStory, iteration: BoardStory): boolean => {
  return positionInParentIteration(story, iteration.number) !== null
}

export const epicNumbersInIteration = (stories: BoardStory[], iteration: BoardStory): number[] => {
  const epicSet = new Set<number>()
  for (const s of stories) {
    if (!isMemberOfIteration(s, iteration)) continue
    if (typeof s.parent_epic === 'number') epicSet.add(s.parent_epic)
  }
  return Array.from(epicSet).sort((a, b) => a - b)
}
