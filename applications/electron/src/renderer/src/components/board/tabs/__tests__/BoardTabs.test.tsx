import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import BoardTabs from '../BoardTabs'
import { iterationLabel } from '../../iteration/labelSchema'
import { boardTabs } from './BoardTabs.page'
import { buildProps, epicStory, featureStory } from './BoardTabs.fixtures'
import { boardIteration, seedIterations } from '../../iteration/__tests__/Iteration.fixtures'
import { groupOrderFrom, type BoardColumn } from '../../types'
import { resetStateForTest } from '@state/store'

const threeStageColumns: BoardColumn[] = [
  { id: 'icebox', label: 'Icebox' },
  { id: 'ready', label: 'Ready' },
  { id: 'build', label: 'Build' },
  { id: 'review', label: 'Review' },
  { id: 'harden', label: 'Harden' },
  { id: 'acceptance', label: 'Acceptance' },
  { id: 'accepted', label: 'Accepted' },
  { id: 'retired', label: 'Retired' }
]
afterEach(() => {
  cleanup()
  resetStateForTest()
})
beforeEach(() => {
  window.localStorage.clear()
})

describe('BoardTabs — 3-tab Now/Plan/Done shape', () => {
  test('renders 3 tabs with canonical labels Now / Plan / Done', () => {
    render(<BoardTabs {...buildProps([])} />)
    expect(boardTabs.hasTab('Now')).toBe(true)
    expect(boardTabs.hasTab('Plan')).toBe(true)
    expect(boardTabs.hasTab('Done')).toBe(true)
  })

  test('default tab is Now on first render (no persisted state)', () => {
    render(<BoardTabs {...buildProps([])} />)
    expect(boardTabs.isActive('now')).toBe(true)
    expect(boardTabs.isActive('plan')).toBe(false)
    expect(boardTabs.isActive('done')).toBe(false)
  })

  test(': NOW sections show the current iteration members per state, not the global ready column', () => {
    seedIterations([
      boardIteration({
        id: 1552,
        title: 'Iteration Carina',
        is_current: true,
        stories: [9001, 9002]
      })
    ])
    const m1 = featureStory({
      id: 9001,
      state: 'ready'
    })
    const m2 = featureStory({
      id: 9002,
      state: 'in-progress'
    })
    const nonMember = featureStory({ id: 9999, state: 'ready' })
    render(<BoardTabs {...buildProps([m1, m2, nonMember])} />)
    expect(boardTabs.panelHasStory('now', 9001)).toBe(true)
    expect(boardTabs.panelHasStory('now', 9002)).toBe(true)
    expect(boardTabs.panelHasStory('now', 9999)).toBe(false)
  })

  test(': NOW shows the roadmap-view (what is coming) when none is current — not a dead board', () => {
    seedIterations([boardIteration({ id: 1, title: 'Iteration A' })])
    render(<BoardTabs {...buildProps([])} />)
    expect(boardTabs.nowHasRoadmap()).toBe(true)
  })

  test(': Now tab renders the current iteration member directly (no dedicated anchor-box)', () => {
    seedIterations([
      boardIteration({
        id: 1500,
        title: 'Iteration Sextans',
        is_current: true,
        stories: [9101]
      })
    ])
    const inFlightChild = featureStory({
      id: 9101,
      state: 'in-progress'
    })
    render(<BoardTabs {...buildProps([inFlightChild])} />)
    expect(boardTabs.panelHasStory('now', 9101)).toBe(true)
  })

  test('cont.: Now tab Epic-card shows only Epics belonging to current iteration', () => {
    seedIterations([
      boardIteration({
        id: 1500,
        title: 'Iteration Sextans',
        is_current: true,
        stories: [9101]
      })
    ])
    const epicIn = epicStory({ id: 8001, title: 'Epic In' })
    const epicOut = epicStory({ id: 8002, title: 'Epic Out' })
    const memberOfIter = featureStory({
      id: 9101,
      parent_epic: 8001
    })
    const memberOfOther = featureStory({
      id: 9102,
      parent_epic: 8002
    })
    render(<BoardTabs {...buildProps([epicIn, epicOut, memberOfIter, memberOfOther])} />)
    expect(boardTabs.epicTitleCount('Epic In')).toBeGreaterThan(0)
    expect(boardTabs.epicTitleCount('Epic Out')).toBe(1)
  })

  test('Plan tab renders active Epics, the Iteration component, and the icebox section', async () => {
    seedIterations([boardIteration({ id: 1500, title: 'Iteration Sextans' })])
    const epic = epicStory({ id: 8001, title: 'Epic Foo' })
    render(<BoardTabs {...buildProps([epic])} />)
    await boardTabs.selectTab('plan')
    expect(boardTabs.panelHasGroup('plan', 'Icebox')).toBe(true)
    expect(boardTabs.panelHasIteration('plan', 'Iteration Sextans')).toBe(true)
  })

  test('Done tab renders ACCEPTED + RETIRED sections, not the ready column', async () => {
    render(<BoardTabs {...buildProps([])} />)
    await boardTabs.selectTab('done')
    expect(boardTabs.panelHasGroup('done', 'Accepted')).toBe(true)
    expect(boardTabs.panelHasGroup('done', 'Retired')).toBe(true)
    expect(boardTabs.panelHasGroup('done', 'Ready')).toBe(false)
  })

  test('clicking Plan tab activates it; previously-active Now becomes inactive and hidden', async () => {
    render(<BoardTabs {...buildProps([])} />)
    await boardTabs.selectTab('plan')
    expect(boardTabs.isActive('plan')).toBe(true)
    expect(boardTabs.isActive('now')).toBe(false)
    expect(boardTabs.isPanelVisible('plan')).toBe(true)
    expect(boardTabs.isPanelVisible('now')).toBe(false)
  })

  test('tab choice persists across remounts via localStorage', async () => {
    const props = buildProps([])
    const first = render(<BoardTabs {...props} />)
    await boardTabs.selectTab('done')
    first.unmount()
    render(<BoardTabs {...props} />)
    expect(boardTabs.isActive('done')).toBe(true)
  })

  test('a current-iteration member appears in Now, not in Done', async () => {
    seedIterations([
      boardIteration({
        id: 1552,
        title: 'Iteration Carina',
        is_current: true,
        stories: [9101]
      })
    ])
    const member = featureStory({
      id: 9101,
      state: 'ready'
    })
    render(<BoardTabs {...buildProps([member])} />)
    expect(boardTabs.panelHasStory('now', 9101)).toBe(true)
    await boardTabs.selectTab('done')
    expect(boardTabs.panelHasStory('done', 9101)).toBe(false)
  })

  test(': Now renders the configured middle columns (build/review/harden) as headers in config order', () => {
    seedIterations([
      boardIteration({ id: 1600, title: 'Iteration Lyra', is_current: true, stories: [] })
    ])
    render(<BoardTabs {...buildProps([], groupOrderFrom(threeStageColumns))} />)
    expect(boardTabs.groupLabelsInOrder('now')).toEqual([
      'Ready',
      'Build',
      'Review',
      'Harden',
      'Acceptance'
    ])
  })

  test(': a member in a configured middle stage buckets into that stage column', () => {
    seedIterations([
      boardIteration({ id: 1601, title: 'Iteration Vega', is_current: true, stories: [9200] })
    ])
    const member = featureStory({ id: 9200, state: 'review' })
    render(<BoardTabs {...buildProps([member], groupOrderFrom(threeStageColumns))} />)
    expect(boardTabs.groupHasStory('now', 'Review', 9200)).toBe(true)
  })

  test(': with no wire columns, Now falls back to the legacy middle (In Progress, Desk Check)', () => {
    seedIterations([
      boardIteration({ id: 1602, title: 'Iteration Orion', is_current: true, stories: [] })
    ])
    render(<BoardTabs {...buildProps([])} />)
    expect(boardTabs.groupLabelsInOrder('now')).toEqual([
      'Ready',
      'In Progress',
      'Desk Check',
      'Acceptance'
    ])
  })

  test('Scenario regression: legacy iteration-{Name}.{N} label still derives current-iter Epic', () => {
    seedIterations([boardIteration({ id: 1500, title: 'Iteration Sextans' })])
    const epicIn = epicStory({ id: 8001, title: 'Epic Legacy' })
    const legacyMember = featureStory({
      id: 9101,
      parent_epic: 8001,
      labels: ['feature', iterationLabel('Sextans', 100)]
    })
    render(<BoardTabs {...buildProps([epicIn, legacyMember])} />)
    expect(boardTabs.epicTitleCount('Epic Legacy')).toBeGreaterThan(0)
  })
})
