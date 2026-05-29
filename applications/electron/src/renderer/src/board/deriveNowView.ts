import { findCurrentIteration } from './currentIteration'
import { isIterationStory, positionInParentIteration } from './iterationMembers'
import type { BoardStory, IterationMembership } from './types'

export interface NowRow {
  story: BoardStory
  membership: IterationMembership
}

export type NowView =
  | { kind: 'current'; iteration: BoardStory; members: NowRow[] }
  | { kind: 'empty' }

export const deriveNowView = (stories: BoardStory[]): NowView => {
  const iteration = findCurrentIteration(stories)
  if (iteration === null) return { kind: 'empty' }
  const members: NowRow[] = []
  for (const story of stories) {
    if (isIterationStory(story)) continue
    const position = positionInParentIteration(story, iteration.number)
    if (position === null) continue
    members.push({ story, membership: { iteration_number: iteration.number, position } })
  }
  return { kind: 'current', iteration, members }
}
