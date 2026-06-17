import { findCurrentIteration } from './currentIteration'
import { deriveNowView } from './deriveNowView'
import { firstUnmetLabel, readinessOf, unmetCount } from './iterationReadiness'
import { isIterationStory, positionInIterationOrder } from './iterationMembers'
import { themeNameFromTitle } from './labelSchema'
import { columnFor, type BoardStory, type ColumnState } from './types'

export interface CloseApproachNudge {
  nextIterationNumber: number
  nextIterationName: string
  activeGapLabel: string
}

const CLOSE_APPROACH_WORK_DONE_RATIO = 0.75

const WORK_DONE_COLUMNS: ReadonlyArray<ColumnState> = ['delivered', 'accepted']

const nextIterationAfter = (current: BoardStory, stories: BoardStory[]): BoardStory | null => {
  const currentOrder = positionInIterationOrder(current)
  if (currentOrder === null) return null
  const successors = stories
    .filter(isIterationStory)
    .filter((it) => it.gh_state !== 'closed')
    .map((it) => ({ it, order: positionInIterationOrder(it) }))
    .filter(
      (c): c is { it: BoardStory; order: number } => c.order !== null && c.order > currentOrder
    )
    .sort((a, b) => a.order - b.order)
  return successors[0]?.it ?? null
}

const columnOf = (story: BoardStory): ColumnState =>
  columnFor(story.state, story.gh_state === 'closed')

const isDraining = (current: BoardStory, stories: BoardStory[]): boolean => {
  const view = deriveNowView(stories)
  if (view.kind !== 'current' || view.iteration.number !== current.number) return false
  const active = view.members.filter((row) => columnOf(row.story) !== 'retired')
  if (active.length === 0) return false
  const workDone = active.filter((row) => WORK_DONE_COLUMNS.includes(columnOf(row.story))).length
  return workDone / active.length >= CLOSE_APPROACH_WORK_DONE_RATIO
}

export const deriveCloseApproachNudge = (stories: BoardStory[]): CloseApproachNudge | null => {
  const current = findCurrentIteration(stories)
  if (current === null) return null
  if (!isDraining(current, stories)) return null

  const next = nextIterationAfter(current, stories)
  if (next === null) return null

  const readiness = readinessOf(next)
  if (unmetCount(readiness) === 0) return null

  return {
    nextIterationNumber: next.number,
    nextIterationName: themeNameFromTitle(next.title),
    activeGapLabel: firstUnmetLabel(readiness) ?? 'not ready'
  }
}
