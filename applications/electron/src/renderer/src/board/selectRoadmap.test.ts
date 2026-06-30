import { describe, expect, it } from 'vitest'
import {
  selectRoadmap,
  parseOutcomeWhy,
  type RoadmapBand,
  type RoadmapOutcome,
  type RoadmapOutcomeData
} from './selectRoadmap'
import type { Horizon, OutcomeReadiness } from './types'
const outcome = (overrides: Partial<RoadmapOutcomeData>): RoadmapOutcomeData => ({
  id: 1,
  title: 'An outcome',
  body: '',
  horizon: 'now',
  horizon_order: null,
  outcome_readiness: null,
  iterations: [],
  ...overrides
})

const bandsOf = (...outcomes: RoadmapOutcomeData[]): RoadmapBand[] =>
  selectRoadmap({ outcomes }).bands

const readiness = (over: Partial<OutcomeReadiness>): OutcomeReadiness => ({
  iteration_count: 0,
  gap_count: 0,
  all_ready: false,
  ...over
})

describe('selectRoadmap — outcomes grouped by horizon', () => {
  it('groups outcomes into Now/Next/Later bands in that fixed order', () => {
    const bands = bandsOf(
      outcome({ id: 3, title: 'Later thing', horizon: 'later' }),
      outcome({ id: 1, title: 'Now thing', horizon: 'now' }),
      outcome({ id: 2, title: 'Next thing', horizon: 'next' })
    )
    expect(bands.map((b) => b.horizon)).toEqual<Horizon[]>(['now', 'next', 'later'])
    expect(bands.map((b) => b.label)).toEqual(['Now', 'Next', 'Later'])
    expect(bands[0].outcomes[0].name).toBe('Now thing')
  })

  it('omits a horizon band that has no outcomes (compact-curated, not empty headers)', () => {
    expect(bandsOf(outcome({ id: 1, horizon: 'now' })).map((b) => b.horizon)).toEqual<Horizon[]>([
      'now'
    ])
  })

  it('orders outcomes within a band deterministically by issue-number', () => {
    const bands = bandsOf(
      outcome({ id: 9, title: 'nine', horizon: 'now' }),
      outcome({ id: 4, title: 'four', horizon: 'now' })
    )
    expect(bands[0].outcomes.map((o) => o.id)).toEqual([4, 9])
  })

  it('renders the plain outcome name — never substrate-jargon', () => {
    const bands = bandsOf(
      outcome({ id: 42, title: 'The team runs on its own infra', horizon: 'now' })
    )
    expect(bands[0].outcomes[0].name).toBe('The team runs on its own infra')
  })

  it('treats an absent horizon as Later', () => {
    expect(bandsOf(outcome({ horizon: null }))[0].horizon).toBe('later')
  })

  it('orders outcomes within a band by horizon_order, then number', () => {
    const [band] = bandsOf(
      outcome({ id: 10, title: 'second', horizon: 'now', horizon_order: 2 }),
      outcome({ id: 20, title: 'first', horizon: 'now', horizon_order: 1 })
    )
    expect(band.outcomes.map((o) => o.name)).toEqual(['first', 'second'])
  })

  it('sinks an outcome with no horizon_order below ordered peers, ties broken by number', () => {
    const [band] = bandsOf(
      outcome({ id: 5, title: 'unordered', horizon: 'now', horizon_order: null }),
      outcome({ id: 9, title: 'ordered', horizon: 'now', horizon_order: 1 })
    )
    expect(band.outcomes.map((o) => o.name)).toEqual(['ordered', 'unordered'])
  })
})

describe('selectRoadmap readiness — live-derived from outcome_readiness, rolled up', () => {
  const stateFor = (r: OutcomeReadiness | null, horizon: Horizon = 'now'): RoadmapOutcome =>
    bandsOf(outcome({ horizon, outcome_readiness: r }))[0].outcomes[0]

  it('all_ready → ready', () => {
    expect(stateFor(readiness({ iteration_count: 1, all_ready: true })).readinessState).toBe(
      'ready'
    )
  })

  it('gap_count > 0 → gap, carrying the count', () => {
    const o = stateFor(readiness({ iteration_count: 2, gap_count: 1 }))
    expect(o.readinessState).toBe('gap')
    expect(o.gapCount).toBe(1)
  })

  it('has building iterations, not all-ready → gap (convene-readiness has no 4th state)', () => {
    expect(stateFor(readiness({ iteration_count: 1, all_ready: false })).readinessState).toBe('gap')
  })

  it('no building iterations (or absent readiness) → none (unmapped, no chip)', () => {
    expect(stateFor(readiness({ iteration_count: 0 })).readinessState).toBe('none')
    expect(stateFor(null).readinessState).toBe('none')
  })

  it('derives the card tone: gap > ready > horizon-fallback (now vs future)', () => {
    expect(stateFor(readiness({ iteration_count: 2, gap_count: 1 })).tone).toBe('gap')
    expect(stateFor(readiness({ iteration_count: 1, all_ready: true })).tone).toBe('ready')
    expect(stateFor(null, 'now').tone).toBe('now')
    expect(stateFor(null, 'later').tone).toBe('future')
  })
})

describe('parseOutcomeWhy — labelled-line body convention', () => {
  it('extracts benefit, success-metric, confidence, value from labelled lines', () => {
    const why = parseOutcomeWhy(
      '## Title\n\n**Benefit:** any agent plugs in\n**Success-metric:** 0 Claude-only paths\n**Confidence:** High\n**Value:** high\n'
    )
    expect(why.benefit).toBe('any agent plugs in')
    expect(why.metric).toBe('0 Claude-only paths')
    expect(why.confidence).toBe('High')
    expect(why.value).toBe('high')
  })

  it('tolerates a plain (non-bold) label and a "Metric:" alias', () => {
    const why = parseOutcomeWhy('Benefit: x\nMetric: y')
    expect(why.benefit).toBe('x')
    expect(why.metric).toBe('y')
  })

  it('OMITS an absent field (no labelled line) — distinct from present-but-empty', () => {
    const why = parseOutcomeWhy('**Benefit:** only this')
    expect('benefit' in why).toBe(true)
    expect('metric' in why).toBe(false)
    expect('confidence' in why).toBe(false)
    expect('value' in why).toBe(false)
  })

  it('keeps a PRESENT-but-empty field as "" (the graceful no-parse-fail case → renders —)', () => {
    const why = parseOutcomeWhy('**Benefit:**\n**Confidence:** High')
    expect('benefit' in why).toBe(true)
    expect(why.benefit).toBe('')
    expect(why.confidence).toBe('High')
  })

  it('attaches the parsed why to each outcome in the view', () => {
    const bands = bandsOf(
      outcome({ horizon: 'now', body: '**Benefit:** plugs in\n**Confidence:** High' })
    )
    expect(bands[0].outcomes[0].why.benefit).toBe('plugs in')
    expect(bands[0].outcomes[0].why.confidence).toBe('High')
    expect('value' in bands[0].outcomes[0].why).toBe(false)
  })
})
