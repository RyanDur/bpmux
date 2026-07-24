import { findCurrentIteration } from './currentIteration'
import { deriveNowView } from './deriveNowView'
import { firstUnmetLabel, readinessOf, unmetCount } from './iterationReadiness'
import { iterationNameFromTitle } from './labelSchema'
import type { BoardIterationData } from './selectIterations'
import { columnFor, type Story, type ColumnState } from './types'

export type CloseApproachNudge = {
  nextIterationId: number
  nextIterationName: string
  activeGapLabel: string
}

const CLOSE_APPROACH_WORK_DONE_RATIO = 0.75

const WORK_DONE_COLUMNS: ReadonlyArray<ColumnState> = ['acceptance', 'accepted']

const nextIterationAfter = (
  current: BoardIterationData,
  iterations: BoardIterationData[]
): BoardIterationData | null => {
  const currentOrder = current.iteration_order
  if (currentOrder === null) return null
  const successors = iterations
    .filter((it) => !it.complete)
    .map((it) => ({ it, order: it.iteration_order }))
    .filter(
      (c): c is { it: BoardIterationData; order: number } =>
        c.order !== null && c.order > currentOrder
    )
    .sort((a, b) => a.order - b.order)
  return successors[0]?.it ?? null
}

const columnOf = (story: Story): ColumnState => columnFor(story.state)

const isDraining = (
  current: BoardIterationData,
  iterations: BoardIterationData[],
  stories: Story[]
): boolean => {
  const view = deriveNowView(iterations, stories)
  if (view.kind !== 'current' || view.iteration.id !== current.id) return false
  const active = view.members.filter((row) => columnOf(row.story) !== 'retired')
  if (active.length === 0) return false
  const workDone = active.filter((row) => WORK_DONE_COLUMNS.includes(columnOf(row.story))).length
  return workDone / active.length >= CLOSE_APPROACH_WORK_DONE_RATIO
}

export const deriveCloseApproachNudge = (
  iterations: BoardIterationData[],
  stories: Story[]
): CloseApproachNudge | null => {
  const current = findCurrentIteration(iterations)
  if (current === null) return null
  if (!isDraining(current, iterations, stories)) return null

  const next = nextIterationAfter(current, iterations)
  if (next === null) return null

  const readiness = readinessOf(next)
  if (unmetCount(readiness) === 0) return null

  return {
    nextIterationId: next.id,
    nextIterationName: iterationNameFromTitle(next.title),
    activeGapLabel: firstUnmetLabel(readiness) ?? 'not ready'
  }
}
