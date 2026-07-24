import type { Horizon, IterationReadiness, OutcomeReadiness } from '../types'
export type RoadmapIteration = {
  id: number
  name: string
  position: number
  readiness: IterationReadiness
}

type RoadmapReadinessState = 'ready' | 'gap' | 'none'

export type OutcomeState = 'done' | 'ready' | 'needs-attention' | 'planned'

type OutcomeCardTone = 'now' | 'ready' | 'gap' | 'future'

export type OutcomeWhy = {
  benefit?: string
  metric?: string
  confidence?: string
  value?: string
}

export type RoadmapOutcome = {
  id: number
  name: string
  why: OutcomeWhy
  readinessState: RoadmapReadinessState
  gapCount: number
  tone: OutcomeCardTone
  iterations: RoadmapIteration[]
}

type RoadmapBand = {
  horizon: Horizon
  label: string
  outcomes: RoadmapOutcome[]
}

export type Roadmap = {
  bands: RoadmapBand[]
}

export type RoadmapOutcomeData = {
  id: number
  title: string
  body: string
  horizon: Horizon | null
  horizon_order: number | null
  outcome_readiness: OutcomeReadiness | null
  iterations: RoadmapIteration[]
}

export type RoadmapCard = {
  id: number
  name: string
  why: OutcomeWhy
  horizon: Horizon
  state: OutcomeState
  gap_count: number
  tone: OutcomeCardTone
}

const HORIZON_BANDS: { horizon: Horizon; label: string }[] = [
  { horizon: 'now', label: 'Now' },
  { horizon: 'next', label: 'Next' },
  { horizon: 'later', label: 'Later' }
]

const chipStateOf = (state: OutcomeState): RoadmapReadinessState => {
  switch (state) {
    case 'ready':
    case 'done':
      return 'ready'
    case 'needs-attention':
      return 'gap'
    case 'planned':
      return 'none'
  }
}

const toRoadmapOutcome = (card: RoadmapCard, iterations: RoadmapIteration[]): RoadmapOutcome => ({
  id: card.id,
  name: card.name,
  why: card.why,
  readinessState: chipStateOf(card.state),
  gapCount: card.gap_count,
  tone: card.tone,
  iterations
})

export type RoadmapState = {
  roadmap: RoadmapCard[]
  outcomes: RoadmapOutcomeData[]
}

export const selectRoadmap = (state: RoadmapState): Roadmap => {
  const iterationsById = new Map(state.outcomes.map((outcome) => [outcome.id, outcome.iterations]))
  const bands: RoadmapBand[] = []
  for (const { horizon, label } of HORIZON_BANDS) {
    const outcomes = state.roadmap
      .filter((card) => card.horizon === horizon)
      .map((card) => toRoadmapOutcome(card, iterationsById.get(card.id) ?? []))
    if (outcomes.length > 0) bands.push({ horizon, label, outcomes })
  }
  return { bands }
}
