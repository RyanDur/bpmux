import type { BoardIterationData } from './selectIterations'
import { dispatchSession } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

export const boardIteration = (overrides: Partial<BoardIterationData>): BoardIterationData => ({
  number: 9000,
  title: 'Iteration Lyra',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories: [],
  ...overrides
})

export const seedIterations = (iterations: BoardIterationData[]): void =>
  dispatchSession(mirrorDeltaReceived({ domain: 'iterations', iterations }, '2026-06-28T00:00:00Z'))
