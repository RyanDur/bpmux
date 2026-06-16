import { cleanup, render, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import type { BoardStory, IterationReadiness } from './types'

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

const buildingIteration = (
  number: number,
  title: string,
  position: number,
  readiness: IterationReadiness,
  outcomeNumber = 1
): BoardStory => ({
  number,
  title,
  state: 'in-progress',
  labels: ['iteration'],
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: 'goal',
  outcome_membership: { outcome_number: outcomeNumber, position },
  readiness
})

const checks = (
  drafted: boolean,
  stories_ordered: boolean,
  review_closed: boolean,
  dev_ok: boolean
): IterationReadiness => ({ drafted, stories_ordered, review_closed, dev_ok })

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

describe('#1737 Roadmap drill-in — focus an outcome → its iterations building it, in order', () => {
  const stories = (): BoardStory[] => [
    outcome({ number: 1, title: 'You see what is coming', horizon: 'now' }),
    buildingIteration(10, 'Crux', 1, checks(true, true, false, false)),
    buildingIteration(20, 'Lyra', 2, checks(false, false, false, false)),
    { ...outcome({ number: 2 }), labels: ['epic'], title: 'a board epic' }
  ]

  it('is collapsed by default — the drillbody is NOT in the DOM (render-on-demand)', () => {
    const { queryByTestId } = render(<Roadmap stories={stories()} />)
    expect(queryByTestId('roadmap-drill-1')).toBeNull()
    expect(queryByTestId('roadmap-drill-iteration-10')).toBeNull()
  })

  it('focusing an outcome reveals the SEQUENCE OF ITERATIONS building it, in order, with the CONVENE? banner', async () => {
    const { getByTestId } = render(<Roadmap stories={stories()} />)
    await userEvent.click(getByTestId('roadmap-outcome-toggle-1'))

    const drill = getByTestId('roadmap-drill-1')
    const iterationNames = within(drill)
      .getAllByTestId(/roadmap-drill-iteration-\d+/)
      .map((row) => within(row).getByTestId('roadmap-drill-iteration-name').textContent)
    expect(iterationNames).toEqual(['Crux', 'Lyra'])

    const convene = within(drill).getByTestId('roadmap-convene-1').textContent ?? ''
    expect(convene).toContain('CONVENE?')
    expect(convene).toContain('Crux')
    expect(convene).toContain('designer-review')
  })

  it('the focused iteration shows its tri-state convene-checks; a not-started iteration shows none', async () => {
    const { getByTestId, queryByTestId } = render(<Roadmap stories={stories()} />)
    await userEvent.click(getByTestId('roadmap-outcome-toggle-1'))

    const crux = getByTestId('roadmap-drill-iteration-10')
    expect(within(crux).getByText('designer-review')).toBeTruthy()
    expect(within(crux).getByText('drafted')).toBeTruthy()

    const lyra = getByTestId('roadmap-drill-iteration-20')
    expect(lyra.getAttribute('data-state')).toBe('not-started')
    expect(queryByTestId('roadmap-drill-checks-20')).toBeNull()
  })

  it('surfaces ONLY the iterations building the outcome — never epics (a board grouping)', async () => {
    const { getByTestId } = render(<Roadmap stories={stories()} />)
    await userEvent.click(getByTestId('roadmap-outcome-toggle-1'))
    expect(getByTestId('roadmap-drill-1').textContent).not.toContain('a board epic')
  })

  it('toggling the focused outcome again collapses it — back to the overview', async () => {
    const { getByTestId, queryByTestId } = render(<Roadmap stories={stories()} />)
    await userEvent.click(getByTestId('roadmap-outcome-toggle-1'))
    expect(getByTestId('roadmap-drill-1')).toBeTruthy()
    await userEvent.click(getByTestId('roadmap-outcome-toggle-1'))
    expect(queryByTestId('roadmap-drill-1')).toBeNull()
  })

  it('focuses one outcome at a time — focusing another collapses the first', async () => {
    const multi: BoardStory[] = [
      outcome({ number: 1, title: 'first', horizon: 'now' }),
      buildingIteration(10, 'Crux', 1, checks(true, false, false, false), 1),
      outcome({ number: 2, title: 'second', horizon: 'now' }),
      buildingIteration(30, 'Vega', 1, checks(true, false, false, false), 2)
    ]
    const { getByTestId, queryByTestId } = render(<Roadmap stories={multi} />)
    await userEvent.click(getByTestId('roadmap-outcome-toggle-1'))
    expect(getByTestId('roadmap-drill-1')).toBeTruthy()
    await userEvent.click(getByTestId('roadmap-outcome-toggle-2'))
    expect(queryByTestId('roadmap-drill-1')).toBeNull()
    expect(getByTestId('roadmap-drill-2')).toBeTruthy()
  })
})
