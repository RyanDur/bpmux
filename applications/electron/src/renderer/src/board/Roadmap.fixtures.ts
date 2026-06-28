import type { RoadmapIteration, RoadmapOutcomeData } from './selectRoadmap'
import type { IterationReadiness } from './types'
import { dispatchSession } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

export const outcome = (overrides: Partial<RoadmapOutcomeData>): RoadmapOutcomeData => ({
  number: 1,
  title: 'An outcome',
  body: '',
  horizon: 'now',
  horizon_order: null,
  outcome_readiness: null,
  iterations: [],
  ...overrides
})

export const buildingIteration = (
  number: number,
  name: string,
  position: number,
  readiness: IterationReadiness
): RoadmapIteration => ({ number, name, position, readiness })

export const checks = (
  drafted: boolean,
  stories_ordered: boolean,
  review_closed: boolean,
  dev_ok: boolean
): IterationReadiness => ({ drafted, stories_ordered, review_closed, dev_ok })

export const seedOutcomes = (outcomes: RoadmapOutcomeData[]): void =>
  dispatchSession(mirrorDeltaReceived({ domain: 'outcomes', outcomes }, '2026-06-27T00:00:00Z'))
