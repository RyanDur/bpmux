import { cleanup, render, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import type { BoardStory } from './types'

afterEach(() => cleanup())

const outcome = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 1,
  title: 'An outcome',
  state: 'icebox',
  labels: ['outcome'],
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: '',
  horizon: 'now',
  ...overrides
})

describe('#1704 Roadmap — outcomes by horizon, the overview', () => {
  it('groups outcomes under Now / Next / Later band headings', () => {
    const { getAllByRole, getByTestId } = render(
      <Roadmap
        stories={[
          outcome({ number: 1, title: 'Now outcome', horizon: 'now' }),
          outcome({ number: 2, title: 'Next outcome', horizon: 'next' }),
          outcome({ number: 3, title: 'Later outcome', horizon: 'later' })
        ]}
      />
    )
    expect(getAllByRole('heading', { level: 3 }).map((h) => h.textContent)).toEqual([
      'Now',
      'Next',
      'Later'
    ])
    expect(within(getByTestId('roadmap-band-now')).getByText('Now outcome')).toBeTruthy()
  })

  it('renders the plain outcome name, never substrate-jargon', () => {
    const { getByTestId } = render(
      <Roadmap stories={[outcome({ number: 42, title: 'The team runs on its own infra' })]} />
    )
    const card = getByTestId('roadmap-outcome-42')
    expect(within(card).getByText('The team runs on its own infra')).toBeTruthy()
    expect(card.textContent).not.toContain('#42')
    expect(card.textContent?.toLowerCase()).not.toContain('epic')
  })

  it('shows the benefit as the why-line, omitting it when absent', () => {
    const { getByTestId } = render(
      <Roadmap
        stories={[
          outcome({ number: 1, body: '**Benefit:** any agent plugs in' }),
          outcome({ number: 2, body: '' })
        ]}
      />
    )
    expect(within(getByTestId('roadmap-outcome-1')).getByText('any agent plugs in')).toBeTruthy()
    expect(within(getByTestId('roadmap-outcome-2')).queryByTestId('roadmap-outcome-why')).toBeNull()
  })

  it('renders only present meta fields and OMITS absent ones (no dash-for-absent)', () => {
    const { getByTestId } = render(
      <Roadmap stories={[outcome({ number: 1, body: '**Benefit:** b\n**Confidence:** High' })]} />
    )
    const card = getByTestId('roadmap-outcome-1')
    expect(within(card).getByText('Confidence')).toBeTruthy()
    expect(within(card).getByText('High')).toBeTruthy()
    expect(within(card).queryByText('Metric')).toBeNull()
    expect(within(card).queryByText('Value')).toBeNull()
    expect(card.textContent).not.toContain('—')
  })

  it('renders "—" only for a PRESENT-but-empty meta field (graceful no-parse-fail)', () => {
    const { getByTestId } = render(
      <Roadmap stories={[outcome({ number: 1, body: '**Value:**' })]} />
    )
    const card = getByTestId('roadmap-outcome-1')
    expect(within(card).getByText('Value')).toBeTruthy()
    expect(within(card).getByText('—')).toBeTruthy()
  })

  it('renders the 3 convene-readiness chip states (ready, N-gap, and none = no chip)', () => {
    const { getByTestId } = render(
      <Roadmap
        stories={[
          outcome({
            number: 1,
            outcome_readiness: { iteration_count: 1, gap_count: 0, all_ready: true }
          }),
          outcome({
            number: 2,
            outcome_readiness: { iteration_count: 2, gap_count: 1, all_ready: false }
          }),
          outcome({
            number: 3,
            outcome_readiness: { iteration_count: 0, gap_count: 0, all_ready: false }
          })
        ]}
      />
    )
    expect(
      within(getByTestId('roadmap-outcome-1')).getByTestId('roadmap-outcome-readiness').textContent
    ).toContain('ready')
    expect(
      within(getByTestId('roadmap-outcome-2')).getByTestId('roadmap-outcome-readiness').textContent
    ).toContain('1 gap')
    expect(
      within(getByTestId('roadmap-outcome-3')).queryByTestId('roadmap-outcome-readiness')
    ).toBeNull()
  })

  it('shows the live indicator (readiness is live-derived, not a static dump)', () => {
    const { getByTestId } = render(<Roadmap stories={[outcome({ number: 1 })]} />)
    expect(getByTestId('roadmap-live')).toBeTruthy()
  })

  it('surfaces only outcomes — not epics, iterations, or feature stories', () => {
    const { getByText, queryByText } = render(
      <Roadmap
        stories={[
          outcome({ number: 1, title: 'real outcome' }),
          { ...outcome({ number: 2 }), labels: ['epic'], title: 'an epic' },
          { ...outcome({ number: 3 }), labels: ['iteration'], title: 'an iteration' }
        ]}
      />
    )
    expect(getByText('real outcome')).toBeTruthy()
    expect(queryByText('an epic')).toBeNull()
    expect(queryByText('an iteration')).toBeNull()
  })

  it('renders an empty-state when there are no outcomes', () => {
    const { getByTestId } = render(
      <Roadmap stories={[{ ...outcome({ number: 1 }), labels: ['feature'] }]} />
    )
    expect(getByTestId('roadmap-empty')).toBeTruthy()
  })
})
