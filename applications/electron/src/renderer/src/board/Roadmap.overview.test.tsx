import { cleanup, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { outcome } from './Roadmap.fixtures'

afterEach(() => cleanup())

describe('#1704 Roadmap — outcomes by horizon, the overview', () => {
  it('groups outcomes under Now / Next / Later band headings', () => {
    render(
      <Roadmap
        stories={[
          outcome({ number: 1, title: 'Now outcome', horizon: 'now' }),
          outcome({ number: 2, title: 'Next outcome', horizon: 'next' }),
          outcome({ number: 3, title: 'Later outcome', horizon: 'later' })
        ]}
      />
    )
    expect(roadmap.bandHeadings()).toEqual(['Now', 'Next', 'Later'])
    expect(within(roadmap.band('Now')).getByText('Now outcome')).toBeTruthy()
  })

  it('renders the plain outcome name, never substrate-jargon', () => {
    render(<Roadmap stories={[outcome({ number: 42, title: 'The team runs on its own infra' })]} />)
    const card = roadmap.card('The team runs on its own infra')
    expect(within(card).getByText('The team runs on its own infra')).toBeTruthy()
    expect(card.textContent).not.toContain('#42')
    expect(card.textContent?.toLowerCase()).not.toContain('epic')
  })

  it('shows the benefit as the why-line, omitting it when absent', () => {
    render(
      <Roadmap
        stories={[
          outcome({ number: 1, title: 'Has benefit', body: '**Benefit:** any agent plugs in' }),
          outcome({ number: 2, title: 'No benefit', body: '' })
        ]}
      />
    )
    expect(within(roadmap.card('Has benefit')).getByText('any agent plugs in')).toBeTruthy()
    expect(roadmap.hasBenefit('No benefit')).toBe(false)
  })

  it('renders only present meta fields and OMITS absent ones (no dash-for-absent)', () => {
    render(
      <Roadmap
        stories={[
          outcome({
            number: 1,
            title: 'Meta outcome',
            body: '**Benefit:** b\n**Confidence:** High'
          })
        ]}
      />
    )
    const card = roadmap.card('Meta outcome')
    expect(within(card).getByText('Confidence')).toBeTruthy()
    expect(within(card).getByText('High')).toBeTruthy()
    expect(within(card).queryByText('Metric')).toBeNull()
    expect(within(card).queryByText('Value')).toBeNull()
    expect(card.textContent).not.toContain('—')
  })

  it('renders "—" only for a PRESENT-but-empty meta field (graceful no-parse-fail)', () => {
    render(<Roadmap stories={[outcome({ number: 1, title: 'Empty value', body: '**Value:**' })]} />)
    const card = roadmap.card('Empty value')
    expect(within(card).getByText('Value')).toBeTruthy()
    expect(within(card).getByText('—')).toBeTruthy()
  })

  it('renders the 3 convene-readiness chip states (ready, N-gap, and none = no chip)', () => {
    render(
      <Roadmap
        stories={[
          outcome({
            number: 1,
            title: 'Ready one',
            outcome_readiness: { iteration_count: 1, gap_count: 0, all_ready: true }
          }),
          outcome({
            number: 2,
            title: 'Gap two',
            outcome_readiness: { iteration_count: 2, gap_count: 1, all_ready: false }
          }),
          outcome({
            number: 3,
            title: 'None three',
            outcome_readiness: { iteration_count: 0, gap_count: 0, all_ready: false }
          })
        ]}
      />
    )
    expect(roadmap.readiness('Ready one')?.textContent).toContain('ready')
    expect(roadmap.readiness('Gap two')?.textContent).toContain('1 gap')
    expect(roadmap.readiness('None three')).toBeNull()
  })

  it('shows the live indicator (readiness is live-derived, not a static dump)', () => {
    render(<Roadmap stories={[outcome({ number: 1 })]} />)
    expect(roadmap.liveShown()).toBe(true)
  })

  it('surfaces only outcomes — not epics, iterations, or feature stories', () => {
    render(
      <Roadmap
        stories={[
          outcome({ number: 1, title: 'real outcome' }),
          { ...outcome({ number: 2 }), labels: ['epic'], title: 'an epic' },
          { ...outcome({ number: 3 }), labels: ['iteration'], title: 'an iteration' }
        ]}
      />
    )
    expect(screen.getByText('real outcome')).toBeTruthy()
    expect(screen.queryByText('an epic')).toBeNull()
    expect(screen.queryByText('an iteration')).toBeNull()
  })

  it('renders an empty-state when there are no outcomes', () => {
    render(<Roadmap stories={[{ ...outcome({ number: 1 }), labels: ['feature'] }]} />)
    expect(roadmap.emptyShown()).toBe(true)
  })
})
