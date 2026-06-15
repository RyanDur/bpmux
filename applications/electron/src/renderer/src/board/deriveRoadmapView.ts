import type { BoardStory, Horizon, OutcomeReadiness } from './types'
import { storyTypeFromLabels } from './types'

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
}

export interface RoadmapBand {
  horizon: Horizon
  label: string
  outcomes: RoadmapOutcome[]
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

const toRoadmapOutcome = (story: BoardStory): RoadmapOutcome => {
  const horizon = story.horizon ?? 'later'
  const readinessState = readinessStateOf(story.outcome_readiness)
  return {
    number: story.number,
    name: story.title,
    why: parseOutcomeWhy(story.body),
    readinessState,
    gapCount: story.outcome_readiness?.gap_count ?? 0,
    tone: toneOf(horizon, readinessState)
  }
}

export const deriveRoadmapView = (stories: BoardStory[]): RoadmapBand[] => {
  const outcomes = stories.filter((s) => storyTypeFromLabels(s.labels) === 'outcome')
  const bands: RoadmapBand[] = []
  for (const { horizon, label } of HORIZON_BANDS) {
    const inBand = outcomes
      .filter((s) => s.horizon === horizon)
      .sort((a, b) => a.number - b.number)
      .map(toRoadmapOutcome)
    if (inBand.length > 0) bands.push({ horizon, label, outcomes: inBand })
  }
  return bands
}
