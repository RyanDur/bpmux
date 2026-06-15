import { describe, expect, it } from 'vitest'
import { deriveRoadmapView, parseOutcomeWhy, type RoadmapOutcome } from './deriveRoadmapView'
import type { BoardStory, Horizon, OutcomeReadiness } from './types'

const outcome = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 1,
  title: 'An outcome',
  state: 'icebox',
  labels: ['outcome'],
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: '',
  ...overrides
})

const readiness = (over: Partial<OutcomeReadiness>): OutcomeReadiness => ({
  iteration_count: 0,
  gap_count: 0,
  all_ready: false,
  ...over
})

describe('#1704 deriveRoadmapView — outcomes grouped by horizon', () => {
  it('groups outcomes into Now/Next/Later bands in that fixed order', () => {
    const view = deriveRoadmapView([
      outcome({ number: 3, title: 'Later thing', horizon: 'later' }),
      outcome({ number: 1, title: 'Now thing', horizon: 'now' }),
      outcome({ number: 2, title: 'Next thing', horizon: 'next' })
    ])
    expect(view.map((b) => b.horizon)).toEqual<Horizon[]>(['now', 'next', 'later'])
    expect(view.map((b) => b.label)).toEqual(['Now', 'Next', 'Later'])
    expect(view[0].outcomes[0].name).toBe('Now thing')
  })

  it('omits a horizon band that has no outcomes (compact-curated, not empty headers)', () => {
    const view = deriveRoadmapView([outcome({ number: 1, horizon: 'now' })])
    expect(view.map((b) => b.horizon)).toEqual<Horizon[]>(['now'])
  })

  it('orders outcomes within a band deterministically by issue-number', () => {
    const view = deriveRoadmapView([
      outcome({ number: 9, title: 'nine', horizon: 'now' }),
      outcome({ number: 4, title: 'four', horizon: 'now' })
    ])
    expect(view[0].outcomes.map((o) => o.number)).toEqual([4, 9])
  })

  it('surfaces ONLY outcome-typed stories — not features, epics, or iterations', () => {
    const view = deriveRoadmapView([
      outcome({ number: 1, title: 'real outcome', horizon: 'now' }),
      { ...outcome({ number: 2, horizon: 'now' }), labels: ['feature'] },
      { ...outcome({ number: 3, horizon: 'now' }), labels: ['epic'] },
      { ...outcome({ number: 4, horizon: 'now' }), labels: ['iteration'] }
    ])
    expect(view[0].outcomes.map((o) => o.name)).toEqual(['real outcome'])
  })

  it('renders the plain outcome name — never substrate-jargon', () => {
    const view = deriveRoadmapView([
      outcome({ number: 42, title: 'The team runs on its own infra', horizon: 'now' })
    ])
    expect(view[0].outcomes[0].name).toBe('The team runs on its own infra')
  })
})

describe('#1704 readiness — live-derived from outcome_readiness, rolled up', () => {
  const stateFor = (r: OutcomeReadiness | null, horizon: Horizon = 'now'): RoadmapOutcome =>
    deriveRoadmapView([outcome({ horizon, outcome_readiness: r })])[0].outcomes[0]

  it('all_ready → ready', () => {
    const o = stateFor(readiness({ iteration_count: 1, all_ready: true }))
    expect(o.readinessState).toBe('ready')
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

describe('#1704 parseOutcomeWhy — labelled-line body convention', () => {
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
    const view = deriveRoadmapView([
      outcome({ horizon: 'now', body: '**Benefit:** plugs in\n**Confidence:** High' })
    ])
    expect(view[0].outcomes[0].why.benefit).toBe('plugs in')
    expect(view[0].outcomes[0].why.confidence).toBe('High')
    expect('value' in view[0].outcomes[0].why).toBe(false)
  })
})
