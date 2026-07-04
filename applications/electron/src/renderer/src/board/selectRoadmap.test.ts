import { describe, expect, it } from 'vitest'
import {
  selectRoadmap,
  type RoadmapCard,
  type RoadmapOutcomeData,
  type RoadmapState,
  type OutcomeState
} from './selectRoadmap'
import type { Horizon } from './types'

const card = (overrides: Partial<RoadmapCard>): RoadmapCard => ({
  id: 1,
  name: 'An outcome',
  why: {},
  horizon: 'now',
  state: 'planned',
  gap_count: 0,
  tone: 'now',
  ...overrides
})

const state = (roadmap: RoadmapCard[], outcomes: RoadmapOutcomeData[] = []): RoadmapState => ({
  roadmap,
  outcomes
})

describe('selectRoadmap — bands the served cards by horizon', () => {
  it('groups cards into Now / Next / Later in that fixed order, omitting empty bands', () => {
    const { bands } = selectRoadmap(
      state([
        card({ id: 1, name: 'N', horizon: 'now' }),
        card({ id: 3, name: 'L', horizon: 'later' })
      ])
    )
    expect(bands.map((b) => b.horizon)).toEqual<Horizon[]>(['now', 'later'])
    expect(bands.map((b) => b.label)).toEqual(['Now', 'Later'])
  })

  it('preserves the served within-band order — the backend already ranked the cards', () => {
    const { bands } = selectRoadmap(
      state([
        card({ id: 9, name: 'first', horizon: 'now' }),
        card({ id: 4, name: 'second', horizon: 'now' })
      ])
    )
    expect(bands[0].outcomes.map((o) => o.name)).toEqual(['first', 'second'])
  })
})

describe('selectRoadmap — maps the authoritative state to the readiness chip', () => {
  const chipFor = (state: OutcomeState): string => {
    const gap_count = state === 'needs-attention' ? 2 : 0
    return selectRoadmap({ roadmap: [card({ state, gap_count })], outcomes: [] }).bands[0]
      .outcomes[0].readinessState
  }

  it('reads done as ready (the chip does not distinguish completion yet)', () => {
    expect(chipFor('done')).toBe('ready')
  })

  it('reads ready as ready, needs-attention as gap, planned as none', () => {
    expect(chipFor('ready')).toBe('ready')
    expect(chipFor('needs-attention')).toBe('gap')
    expect(chipFor('planned')).toBe('none')
  })
})

describe('selectRoadmap — joins each card to its outcome iterations for the drill-in', () => {
  it('attaches the iterations of the outcome with the same id', () => {
    const iteration = {
      id: 10,
      name: 'Crux',
      position: 1,
      readiness: { drafted: true, stories_ordered: false, review_closed: false, dev_ok: false }
    }
    const { bands } = selectRoadmap(
      state(
        [card({ id: 7, horizon: 'now' })],
        [
          {
            id: 7,
            title: 'x',
            body: '',
            horizon: 'now',
            horizon_order: null,
            outcome_readiness: null,
            iterations: [iteration]
          }
        ]
      )
    )
    expect(bands[0].outcomes[0].iterations).toEqual([iteration])
  })

  it('leaves iterations empty when no outcome matches the card id', () => {
    const { bands } = selectRoadmap(state([card({ id: 7, horizon: 'now' })]))
    expect(bands[0].outcomes[0].iterations).toEqual([])
  })
})
