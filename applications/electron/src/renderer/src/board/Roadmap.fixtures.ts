import type { RoadmapCard, RoadmapIteration, RoadmapOutcomeData } from './selectRoadmap'
import type { IterationReadiness } from './types'
import { dispatchState } from '../transport/state/store'
import { mirrorDeltaReceived } from '../transport/state/actions'

const ts = '2026-06-27T00:00:00Z'

export const card = (overrides: Partial<RoadmapCard>): RoadmapCard => ({
  id: 1,
  name: 'An outcome',
  why: {},
  horizon: 'now',
  state: 'planned',
  gap_count: 0,
  tone: 'now',
  ...overrides
})

export const outcome = (overrides: Partial<RoadmapOutcomeData>): RoadmapOutcomeData => ({
  id: 1,
  title: 'An outcome',
  body: '',
  horizon: 'now',
  horizon_order: null,
  outcome_readiness: null,
  iterations: [],
  ...overrides
})

export const buildingIteration = (
  id: number,
  name: string,
  position: number,
  readiness: IterationReadiness
): RoadmapIteration => ({ id, name, position, readiness })

export const checks = (
  drafted: boolean,
  stories_ordered: boolean,
  review_closed: boolean,
  dev_ok: boolean
): IterationReadiness => ({ drafted, stories_ordered, review_closed, dev_ok })

export const seedRoadmap = (cards: RoadmapCard[], outcomes: RoadmapOutcomeData[] = []): void => {
  dispatchState(mirrorDeltaReceived({ domain: 'roadmap', roadmap: cards }, ts))
  dispatchState(mirrorDeltaReceived({ domain: 'outcomes', outcomes }, ts))
}
