import type { Horizon, IterationReadiness, OutcomeReadiness } from './types'

export interface RoadmapIteration {
  number: number
  name: string
  position: number
  readiness: IterationReadiness
}

export type RoadmapReadinessState = 'ready' | 'gap' | 'none'

export type OutcomeCardTone = 'now' | 'ready' | 'gap' | 'future'

export interface OutcomeWhy {
  benefit?: string
  metric?: string
  confidence?: string
  value?: string
}

export interface RoadmapOutcome {
  number: number
  name: string
  why: OutcomeWhy
  readinessState: RoadmapReadinessState
  gapCount: number
  tone: OutcomeCardTone
  iterations: RoadmapIteration[]
}

export interface RoadmapBand {
  horizon: Horizon
  label: string
  outcomes: RoadmapOutcome[]
}

export interface Roadmap {
  bands: RoadmapBand[]
}

export interface RoadmapOutcomeData {
  number: number
  title: string
  body: string
  horizon: Horizon | null
  outcome_readiness: OutcomeReadiness | null
  iterations: RoadmapIteration[]
}

const HORIZON_BANDS: { horizon: Horizon; label: string }[] = [
  { horizon: 'now', label: 'Now' },
  { horizon: 'next', label: 'Next' },
  { horizon: 'later', label: 'Later' }
]

const WHY_FIELD_BY_LABEL: Record<string, keyof OutcomeWhy> = {
  benefit: 'benefit',
  'success-metric': 'metric',
  metric: 'metric',
  confidence: 'confidence',
  value: 'value'
}

export const parseOutcomeWhy = (body: string): OutcomeWhy => {
  const why: OutcomeWhy = {}
  for (const rawLine of body.split('\n')) {
    const line = rawLine.replace(/\*\*/g, '').trim()
    const colon = line.indexOf(':')
    if (colon === -1) continue
    const field = WHY_FIELD_BY_LABEL[line.slice(0, colon).trim().toLowerCase()]
    if (field === undefined || field in why) continue
    why[field] = line.slice(colon + 1).trim()
  }
  return why
}

const readinessStateOf = (
  readiness: OutcomeReadiness | null | undefined
): RoadmapReadinessState => {
  if (!readiness || readiness.iteration_count === 0) return 'none'
  if (readiness.all_ready) return 'ready'
  return 'gap'
}

const toneOf = (horizon: Horizon, state: RoadmapReadinessState): OutcomeCardTone => {
  if (state === 'gap') return 'gap'
  if (state === 'ready') return 'ready'
  return horizon === 'now' ? 'now' : 'future'
}

const toRoadmapOutcome = (outcome: RoadmapOutcomeData): RoadmapOutcome => {
  const horizon = outcome.horizon ?? 'later'
  const readinessState = readinessStateOf(outcome.outcome_readiness)
  return {
    number: outcome.number,
    name: outcome.title,
    why: parseOutcomeWhy(outcome.body),
    readinessState,
    gapCount: outcome.outcome_readiness?.gap_count ?? 0,
    tone: toneOf(horizon, readinessState),
    iterations: outcome.iterations
  }
}

export interface RoadmapState {
  outcomes: RoadmapOutcomeData[]
}

export const selectRoadmap = (state: RoadmapState): Roadmap => {
  const bands: RoadmapBand[] = []
  for (const { horizon, label } of HORIZON_BANDS) {
    const inBand = state.outcomes
      .filter((o) => (o.horizon ?? 'later') === horizon)
      .sort((a, b) => a.number - b.number)
      .map(toRoadmapOutcome)
    if (inBand.length > 0) bands.push({ horizon, label, outcomes: inBand })
  }
  return { bands }
}
