import type { BoardIterationData } from './selectIterations'
import type { BoardStory } from './types'

export const findCurrentIteration = (
  iterations: BoardIterationData[]
): BoardIterationData | null => {
  return iterations.find((it) => !it.complete && it.is_current) ?? null
}

export const isMemberOfIteration = (story: BoardStory, iteration: BoardIterationData): boolean => {
  return iteration.stories.includes(story.number)
}

export const epicNumbersInIteration = (
  stories: BoardStory[],
  iteration: BoardIterationData
): number[] => {
  const epicSet = new Set<number>()
  for (const s of stories) {
    if (!isMemberOfIteration(s, iteration)) continue
    if (typeof s.parent_epic === 'number') epicSet.add(s.parent_epic)
  }
  return Array.from(epicSet).sort((a, b) => a - b)
}
