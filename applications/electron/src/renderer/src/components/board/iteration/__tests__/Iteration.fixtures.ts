import type { BoardIterationData } from '../selectIterations'
import { dispatchState } from '@state/store'
import { mirrorDeltaReceived } from '@state/actions'

export const boardIteration = (overrides: Partial<BoardIterationData>): BoardIterationData => ({
  id: 9000,
  title: 'Iteration Lyra',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories: [],
  ...overrides
})

export const seedIterations = (iterations: BoardIterationData[]): void =>
  dispatchState(mirrorDeltaReceived({ domain: 'iterations', iterations }, '2026-06-28T00:00:00Z'))
