import { findCurrentIteration } from '../iteration/currentIteration'
import {
  buildMembershipIndex,
  isIterationStory,
  positionInParentIteration
} from '../iterationMembers'
import type { BoardIterationData } from '../iteration/selectIterations'
import type { Story, IterationMembership } from '../types'

export type NowRow = {
  story: Story
  membership: IterationMembership
}

export type NowView =
  | { kind: 'current'; iteration: BoardIterationData; members: NowRow[] }
  | { kind: 'empty' }

export const deriveNowView = (iterations: BoardIterationData[], stories: Story[]): NowView => {
  const iteration = findCurrentIteration(iterations)
  if (iteration === null) return { kind: 'empty' }
  const index = buildMembershipIndex(iterations)
  const members: NowRow[] = []
  for (const story of stories) {
    if (isIterationStory(story)) continue
    const position = positionInParentIteration(index, story.id, iteration.id)
    if (position === null) continue
    members.push({ story, membership: { iteration_id: iteration.id, position } })
  }
  return { kind: 'current', iteration, members }
}
