import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/react'
import BoardTabs from './BoardTabs'
import { iterationLabel } from './labelSchema'
import { groupStories, type BoardStory, type ColumnState } from './types'

afterEach(() => cleanup())
beforeEach(() => {
  window.localStorage.clear()
})

const iterationStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9000,
  title: 'Iteration Sextans',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration', 'iteration-order.100'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const epicStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 8000,
  title: 'Epic Foo',
  state: 'in-progress',
  gh_state: 'open',
  labels: ['epic'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const featureStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9100,
  title: 'a feature',
  state: 'ready',
  gh_state: 'open',
  labels: ['feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const outcomeStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 7000,
  title: 'The team runs on its own infra',
  state: 'icebox',
  gh_state: 'open',
  labels: ['outcome'],
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: '**Benefit:** any agent plugs in',
  horizon: 'now',
  ...overrides
})

const buildProps = (stories: BoardStory[]): Parameters<typeof BoardTabs>[0] => {
  const epics = stories.filter((s) => s.labels.includes('epic'))
  const epicTitleByNumber = new Map(epics.map((e) => [e.number, e.title]))
  const iterationContainerStories = stories.filter(
    (s) => s.labels.includes('iteration') || s.state === 'icebox'
  )
  const flat = stories.filter((s) => !s.labels.includes('epic') && s.state !== 'icebox')
  const grouped = groupStories(flat)
  return {
    stories,
    epics,
    epicTitleByNumber,
    iterationContainerStories,
    grouped,
    loading: false,
    effectiveSelected: null,
    collapsed: {
      icebox: false,
      ready: false,
      'in-progress': false,
      'desk-check': false,
      delivered: false,
      accepted: true,
      retired: true
    } as Record<ColumnState, boolean>,
    onToggleCollapse: () => {},
    onOpen: () => {},
    onOpenTimeline: () => {},
    onSelect: () => {},
    onReorder: () => {},
    onReorderAnnounce: () => {}
  }
}

describe('#1704 BoardTabs — 4-tab Now/Plan/Roadmap/Done shape (additive Roadmap)', () => {
  test('Scenario 1: renders 4 tabs with canonical labels Now / Plan / 🗺 Roadmap / Done', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([])} />)
    expect(getByTestId('board-tab-now').textContent).toBe('Now')
    expect(getByTestId('board-tab-plan').textContent).toBe('Plan')
    expect(getByTestId('board-tab-roadmap').textContent).toBe('🗺 Roadmap')
    expect(getByTestId('board-tab-done').textContent).toBe('Done')
  })

  test('Scenario 2: default tab is Now on first render (no persisted state)', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([])} />)
    const nowRadio = getByTestId('board-tab-now').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    const planRadio = getByTestId('board-tab-plan').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    const roadmapRadio = getByTestId('board-tab-roadmap').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    const doneRadio = getByTestId('board-tab-done').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    expect(nowRadio.checked).toBe(true)
    expect(planRadio.checked).toBe(false)
    expect(roadmapRadio.checked).toBe(false)
    expect(doneRadio.checked).toBe(false)
  })

  test('#1704: Roadmap tab mounts the outcome-roadmap overview, grouped by horizon', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([outcomeStory({ number: 7000 })])} />)
    const roadmapPanel = getByTestId('board-tabpanel-roadmap')
    expect(roadmapPanel.querySelector('[data-test="board-tabpanel-roadmap-content"]')).toBeTruthy()
    expect(roadmapPanel.querySelector('[data-test="roadmap-band-now"]')).toBeTruthy()
    expect(roadmapPanel.querySelector('[data-test="roadmap-outcome-7000"]')).toBeTruthy()
  })

  test('#1704: the Plan tab survives the additive Roadmap (icebox + iterations still mounted)', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const { getByTestId } = render(<BoardTabs {...buildProps([iter])} />)
    const planPanel = getByTestId('board-tabpanel-plan')
    expect(planPanel.querySelector('[data-test="board-group-icebox"]')).toBeTruthy()
    expect(planPanel.querySelector('[data-test="board-iteration-1500"]')).toBeTruthy()
  })

  test('#1549: NOW sections show the current iteration members per state, not the global ready column', () => {
    const carina = iterationStory({ number: 1552, title: 'Iteration Carina', is_current: true })
    const m1 = featureStory({
      number: 9001,
      state: 'ready',
      iteration_membership: { iteration_number: 1552, position: 100 }
    })
    const m2 = featureStory({
      number: 9002,
      state: 'in-progress',
      iteration_membership: { iteration_number: 1552, position: 200 }
    })
    const nonMember = featureStory({ number: 9999, state: 'ready' })
    const { getByTestId } = render(<BoardTabs {...buildProps([carina, m1, m2, nonMember])} />)
    const now = getByTestId('board-tabpanel-now')
    expect(now.querySelector('[data-test="board-story-9001"]')).toBeTruthy()
    expect(now.querySelector('[data-test="board-story-9002"]')).toBeTruthy()
    expect(now.querySelector('[data-test="board-story-9999"]')).toBeNull()
  })

  test('#1549: NOW shows "No iteration designated" when none is current', () => {
    const open = iterationStory({ number: 1, title: 'A', gh_state: 'open' })
    const { getByTestId } = render(<BoardTabs {...buildProps([open])} />)
    expect(getByTestId('board-now-empty').textContent).toContain('No iteration designated')
  })

  test('#1510: Now tab no longer renders the dedicated anchor-box (iteration-context relocated to the board-header)', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans', is_current: true })
    const inFlightChild = featureStory({
      number: 9101,
      state: 'in-progress',
      iteration_membership: { iteration_number: 1500, position: 100 }
    })
    const { queryByTestId } = render(<BoardTabs {...buildProps([iter, inFlightChild])} />)
    expect(queryByTestId('board-now-iteration-anchor')).toBeNull()
  })

  test('Scenario 3 cont.: Now tab Epic-card shows only Epics belonging to current iteration', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const epicIn = epicStory({ number: 8001, title: 'Epic In' })
    const epicOut = epicStory({ number: 8002, title: 'Epic Out' })
    const memberOfIter = featureStory({
      number: 9101,
      parent_epic: 8001,
      iteration_membership: { iteration_number: 1500, position: 100 }
    })
    const memberOfOther = featureStory({
      number: 9102,
      parent_epic: 8002,
      iteration_membership: { iteration_number: 9999, position: 100 }
    })
    const { queryAllByText } = render(
      <BoardTabs {...buildProps([iter, epicIn, epicOut, memberOfIter, memberOfOther])} />
    )
    const nowEpicTitles = queryAllByText('Epic In')
    expect(nowEpicTitles.length).toBeGreaterThan(0)
    const nowOutTitle = queryAllByText('Epic Out')
    expect(nowOutTitle.length).toBe(1)
  })

  test('Scenario 4: Plan tab renders all active Epics, Iteration component, and icebox section', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const epic = epicStory({ number: 8001, title: 'Epic Foo' })
    const { getByTestId } = render(<BoardTabs {...buildProps([iter, epic])} />)
    const planPanel = getByTestId('board-tabpanel-plan')
    expect(planPanel.hasAttribute('hidden')).toBe(true)
    expect(planPanel.querySelector('[data-test="board-tabpanel-plan-content"]')).toBeTruthy()
    expect(planPanel.querySelector('[data-test="board-group-icebox"]')).toBeTruthy()
    expect(planPanel.querySelector('[data-test="board-iteration-1500"]')).toBeTruthy()
  })

  test('Scenario 5: Done tab renders ACCEPTED + RETIRED sections', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([])} />)
    const donePanel = getByTestId('board-tabpanel-done')
    expect(donePanel.querySelector('[data-test="board-group-accepted"]')).toBeTruthy()
    expect(donePanel.querySelector('[data-test="board-group-retired"]')).toBeTruthy()
    expect(donePanel.querySelector('[data-test="board-group-ready"]')).toBeNull()
  })

  test('Scenario 6: clicking Plan tab activates it; previously-active Now becomes inactive', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([])} />)
    fireEvent.click(getByTestId('board-tab-plan'))
    const planRadio = getByTestId('board-tab-plan').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    const nowRadio = getByTestId('board-tab-now').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    expect(planRadio.checked).toBe(true)
    expect(nowRadio.checked).toBe(false)
    expect(getByTestId('board-tabpanel-plan').hasAttribute('hidden')).toBe(false)
    expect(getByTestId('board-tabpanel-now').hasAttribute('hidden')).toBe(true)
  })

  test('Scenario 7: tab choice persists across remounts via localStorage', () => {
    const props = buildProps([])
    const first = render(<BoardTabs {...props} />)
    fireEvent.click(first.getByTestId('board-tab-done'))
    first.unmount()
    const second = render(<BoardTabs {...props} />)
    const doneRadio = second
      .getByTestId('board-tab-done')
      .querySelector('input[type="radio"]') as HTMLInputElement
    expect(doneRadio.checked).toBe(true)
  })

  test('Scenario 10: section discrimination — a current-iteration member appears in Now, not Done', () => {
    const carina = iterationStory({ number: 1552, title: 'Iteration Carina', is_current: true })
    const member = featureStory({
      number: 9101,
      state: 'ready',
      iteration_membership: { iteration_number: 1552, position: 100 }
    })
    const { getByTestId } = render(<BoardTabs {...buildProps([carina, member])} />)
    expect(
      getByTestId('board-tabpanel-now').querySelector('[data-test="board-story-9101"]')
    ).toBeTruthy()
    expect(
      getByTestId('board-tabpanel-done').querySelector('[data-test="board-story-9101"]')
    ).toBeNull()
  })

  test('Scenario regression: legacy iteration-{Theme}.{N} label still derives current-iter Epic', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const epicIn = epicStory({ number: 8001, title: 'Epic Legacy' })
    const legacyMember = featureStory({
      number: 9101,
      parent_epic: 8001,
      labels: ['feature', iterationLabel('Sextans', 100)]
    })
    const { queryAllByText } = render(<BoardTabs {...buildProps([iter, epicIn, legacyMember])} />)
    const nowEpicTitles = queryAllByText('Epic Legacy')
    expect(nowEpicTitles.length).toBeGreaterThan(0)
  })
})
