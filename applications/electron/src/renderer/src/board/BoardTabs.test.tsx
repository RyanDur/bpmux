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

describe('#1453 BoardTabs — 3-tab Now/Plan/Done shape', () => {
  test('Scenario 1: renders 3 tabs with canonical labels Now / Plan / Done', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([])} />)
    expect(getByTestId('board-tab-now').textContent).toBe('Now')
    expect(getByTestId('board-tab-plan').textContent).toBe('Plan')
    expect(getByTestId('board-tab-done').textContent).toBe('Done')
  })

  test('Scenario 2: default tab is Now on first render (no persisted state)', () => {
    const { getByTestId } = render(<BoardTabs {...buildProps([])} />)
    expect(getByTestId('board-tab-now').getAttribute('aria-selected')).toBe('true')
    expect(getByTestId('board-tab-plan').getAttribute('aria-selected')).toBe('false')
    expect(getByTestId('board-tab-done').getAttribute('aria-selected')).toBe('false')
  })

  test('Scenario 3: Now tab shows iteration anchor when an open iteration exists', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const { getByTestId } = render(<BoardTabs {...buildProps([iter])} />)
    const anchor = getByTestId('board-now-iteration-anchor')
    expect(anchor.textContent).toContain('Iteration Sextans')
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
    expect(getByTestId('board-tab-plan').getAttribute('aria-selected')).toBe('true')
    expect(getByTestId('board-tab-now').getAttribute('aria-selected')).toBe('false')
    expect(getByTestId('board-tabpanel-plan').hasAttribute('hidden')).toBe(false)
    expect(getByTestId('board-tabpanel-now').hasAttribute('hidden')).toBe(true)
  })

  test('Scenario 7: tab choice persists across remounts via localStorage', () => {
    const props = buildProps([])
    const first = render(<BoardTabs {...props} />)
    fireEvent.click(first.getByTestId('board-tab-done'))
    first.unmount()
    const second = render(<BoardTabs {...props} />)
    expect(second.getByTestId('board-tab-done').getAttribute('aria-selected')).toBe('true')
  })

  test('Scenario 10: section discrimination — ready stories appear only in Now panel', () => {
    const ready = featureStory({ number: 9101, state: 'ready' })
    const { getByTestId } = render(<BoardTabs {...buildProps([ready])} />)
    const nowPanel = getByTestId('board-tabpanel-now')
    const planPanel = getByTestId('board-tabpanel-plan')
    const donePanel = getByTestId('board-tabpanel-done')
    expect(nowPanel.querySelector('[data-test="board-group-ready"]')).toBeTruthy()
    expect(planPanel.querySelector('[data-test="board-group-ready"]')).toBeNull()
    expect(donePanel.querySelector('[data-test="board-group-ready"]')).toBeNull()
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
