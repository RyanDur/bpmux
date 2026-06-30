import { cleanup, render, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { outcome, seedOutcomes } from './Roadmap.fixtures'
import { resetSessionForTest } from '../transport/session/store'

afterEach(() => {
  cleanup()
  resetSessionForTest()
})

describe('#1704 Roadmap — outcomes by horizon, the overview', () => {
  it('groups outcomes under Now / Next / Later band headings', () => {
    seedOutcomes([
      outcome({ id: 1, title: 'Now outcome', horizon: 'now' }),
      outcome({ id: 2, title: 'Next outcome', horizon: 'next' }),
      outcome({ id: 3, title: 'Later outcome', horizon: 'later' })
    ])
    render(<Roadmap />)
    expect(roadmap.bandHeadings()).toEqual(['Now', 'Next', 'Later'])
    expect(within(roadmap.band('Now')).getByText('Now outcome')).toBeTruthy()
  })

  it('renders the plain outcome name, never substrate-jargon', () => {
    seedOutcomes([outcome({ id: 42, title: 'The team runs on its own infra' })])
    render(<Roadmap />)
    const card = roadmap.card('The team runs on its own infra')
    expect(within(card).getByText('The team runs on its own infra')).toBeTruthy()
    expect(card.textContent).not.toContain('#42')
    expect(card.textContent?.toLowerCase()).not.toContain('epic')
  })

  it('shows the benefit as the why-line, omitting it when absent', () => {
    seedOutcomes([
      outcome({ id: 1, title: 'Has benefit', body: '**Benefit:** any agent plugs in' }),
      outcome({ id: 2, title: 'No benefit', body: '' })
    ])
    render(<Roadmap />)
    expect(within(roadmap.card('Has benefit')).getByText('any agent plugs in')).toBeTruthy()
    expect(roadmap.hasBenefit('No benefit')).toBe(false)
  })

  it('renders only present meta fields and OMITS absent ones (no dash-for-absent)', () => {
    seedOutcomes([
      outcome({ id: 1, title: 'Meta outcome', body: '**Benefit:** b\n**Confidence:** High' })
    ])
    render(<Roadmap />)
    const card = roadmap.card('Meta outcome')
    expect(within(card).getByText('Confidence')).toBeTruthy()
    expect(within(card).getByText('High')).toBeTruthy()
    expect(within(card).queryByText('Metric')).toBeNull()
    expect(within(card).queryByText('Value')).toBeNull()
    expect(card.textContent).not.toContain('—')
  })

  it('renders "—" only for a PRESENT-but-empty meta field (graceful no-parse-fail)', () => {
    seedOutcomes([outcome({ id: 1, title: 'Empty value', body: '**Value:**' })])
    render(<Roadmap />)
    const card = roadmap.card('Empty value')
    expect(within(card).getByText('Value')).toBeTruthy()
    expect(within(card).getByText('—')).toBeTruthy()
  })

  it('renders the 3 convene-readiness chip states (ready, N-gap, and none = no chip)', () => {
    seedOutcomes([
      outcome({
        id: 1,
        title: 'Ready one',
        outcome_readiness: { iteration_count: 1, gap_count: 0, all_ready: true }
      }),
      outcome({
        id: 2,
        title: 'Gap two',
        outcome_readiness: { iteration_count: 2, gap_count: 1, all_ready: false }
      }),
      outcome({
        id: 3,
        title: 'None three',
        outcome_readiness: { iteration_count: 0, gap_count: 0, all_ready: false }
      })
    ])
    render(<Roadmap />)
    expect(roadmap.readiness('Ready one')?.textContent).toContain('ready')
    expect(roadmap.readiness('Gap two')?.textContent).toContain('1 gap')
    expect(roadmap.readiness('None three')).toBeNull()
  })

  it('shows the live indicator (readiness is live-derived, not a static dump)', () => {
    seedOutcomes([outcome({ id: 1 })])
    render(<Roadmap />)
    expect(roadmap.liveShown()).toBe(true)
  })

  it('renders an empty-state when there are no outcomes', () => {
    seedOutcomes([])
    render(<Roadmap />)
    expect(roadmap.emptyShown()).toBe(true)
  })
})
