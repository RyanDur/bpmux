import { findCurrentIteration } from './currentIteration'
import { isIterationStory, positionInParentIteration } from './iterationMembers'
import type { BoardIterationData } from './selectIterations'
import type { BoardStory, IterationMembership } from './types'

export interface NowRow {
  story: BoardStory
  membership: IterationMembership
}

export type NowView =
  | { kind: 'current'; iteration: BoardIterationData; members: NowRow[] }
  | { kind: 'empty' }

export const deriveNowView = (iterations: BoardIterationData[], stories: BoardStory[]): NowView => {
  const iteration = findCurrentIteration(iterations)
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
