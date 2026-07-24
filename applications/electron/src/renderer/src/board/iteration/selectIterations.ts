import type { IterationReadiness } from '../types'
export type BoardIterationData = {
  id: number
  title: string
  iteration_order: number | null
  is_current: boolean
  complete: boolean
  readiness: IterationReadiness
  stories: number[]
}

export type IterationsState = {
  iterations: BoardIterationData[]
}

const orderOf = (iteration: BoardIterationData): number =>
  iteration.iteration_order ?? Number.MAX_SAFE_INTEGER

export const selectIterations = (state: IterationsState): BoardIterationData[] =>
  [...state.iterations].sort((a, b) => orderOf(a) - orderOf(b) || a.id - b.id)
