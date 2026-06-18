import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import BoardTabs from './BoardTabs'
import { iterationLabel } from './labelSchema'
import { boardTabs } from './BoardTabs.page'
import { buildProps, epicStory, featureStory, iterationStory } from './BoardTabs.fixtures'

afterEach(() => cleanup())
beforeEach(() => {
  window.localStorage.clear()
})

describe('#1453 BoardTabs — 3-tab Now/Plan/Done shape', () => {
  test('Scenario 1: renders 3 tabs with canonical labels Now / Plan / Done', () => {
    render(<BoardTabs {...buildProps([])} />)
    expect(boardTabs.hasTab('Now')).toBe(true)
    expect(boardTabs.hasTab('Plan')).toBe(true)
    expect(boardTabs.hasTab('Done')).toBe(true)
  })

  test('Scenario 2: default tab is Now on first render (no persisted state)', () => {
    render(<BoardTabs {...buildProps([])} />)
    expect(boardTabs.isActive('now')).toBe(true)
    expect(boardTabs.isActive('plan')).toBe(false)
    expect(boardTabs.isActive('done')).toBe(false)
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
    render(<BoardTabs {...buildProps([carina, m1, m2, nonMember])} />)
    expect(boardTabs.panelHasStory('now', 9001)).toBe(true)
    expect(boardTabs.panelHasStory('now', 9002)).toBe(true)
    expect(boardTabs.panelHasStory('now', 9999)).toBe(false)
  })

  test('#1684: NOW shows the roadmap-view (what is coming) when none is current — not a dead board', () => {
    const open = iterationStory({ number: 1, title: 'A', gh_state: 'open' })
    render(<BoardTabs {...buildProps([open])} />)
    expect(boardTabs.nowHasRoadmap()).toBe(true)
  })

  test('#1510: Now tab renders the current iteration member directly (no dedicated anchor-box)', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans', is_current: true })
    const inFlightChild = featureStory({
      number: 9101,
      state: 'in-progress',
      iteration_membership: { iteration_number: 1500, position: 100 }
    })
    render(<BoardTabs {...buildProps([iter, inFlightChild])} />)
    expect(boardTabs.panelHasStory('now', 9101)).toBe(true)
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
    render(<BoardTabs {...buildProps([iter, epicIn, epicOut, memberOfIter, memberOfOther])} />)
    expect(boardTabs.epicTitleCount('Epic In')).toBeGreaterThan(0)
    expect(boardTabs.epicTitleCount('Epic Out')).toBe(1)
  })

  test('Scenario 4: Plan tab renders active Epics, the Iteration component, and the icebox section', async () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const epic = epicStory({ number: 8001, title: 'Epic Foo' })
    render(<BoardTabs {...buildProps([iter, epic])} />)
    await boardTabs.selectTab('plan')
    expect(boardTabs.panelHasGroup('plan', 'Icebox')).toBe(true)
    expect(boardTabs.panelHasIteration('plan', 'Iteration Sextans')).toBe(true)
  })

  test('Scenario 5: Done tab renders ACCEPTED + RETIRED sections, not the ready column', async () => {
    render(<BoardTabs {...buildProps([])} />)
    await boardTabs.selectTab('done')
    expect(boardTabs.panelHasGroup('done', 'Accepted')).toBe(true)
    expect(boardTabs.panelHasGroup('done', 'Retired')).toBe(true)
    expect(boardTabs.panelHasGroup('done', 'Ready')).toBe(false)
  })

  test('Scenario 6: clicking Plan tab activates it; previously-active Now becomes inactive and hidden', async () => {
    render(<BoardTabs {...buildProps([])} />)
    await boardTabs.selectTab('plan')
    expect(boardTabs.isActive('plan')).toBe(true)
    expect(boardTabs.isActive('now')).toBe(false)
    expect(boardTabs.isPanelVisible('plan')).toBe(true)
    expect(boardTabs.isPanelVisible('now')).toBe(false)
  })

  test('Scenario 7: tab choice persists across remounts via localStorage', async () => {
    const props = buildProps([])
    const first = render(<BoardTabs {...props} />)
    await boardTabs.selectTab('done')
    first.unmount()
    render(<BoardTabs {...props} />)
    expect(boardTabs.isActive('done')).toBe(true)
  })

  test('Scenario 10: a current-iteration member appears in Now, not in Done', async () => {
    const carina = iterationStory({ number: 1552, title: 'Iteration Carina', is_current: true })
    const member = featureStory({
      number: 9101,
      state: 'ready',
      iteration_membership: { iteration_number: 1552, position: 100 }
    })
    render(<BoardTabs {...buildProps([carina, member])} />)
    expect(boardTabs.panelHasStory('now', 9101)).toBe(true)
    await boardTabs.selectTab('done')
    expect(boardTabs.panelHasStory('done', 9101)).toBe(false)
  })

  test('Scenario regression: legacy iteration-{Theme}.{N} label still derives current-iter Epic', () => {
    const iter = iterationStory({ number: 1500, title: 'Iteration Sextans' })
    const epicIn = epicStory({ number: 8001, title: 'Epic Legacy' })
    const legacyMember = featureStory({
      number: 9101,
      parent_epic: 8001,
      labels: ['feature', iterationLabel('Sextans', 100)]
    })
    render(<BoardTabs {...buildProps([iter, epicIn, legacyMember])} />)
    expect(boardTabs.epicTitleCount('Epic Legacy')).toBeGreaterThan(0)
  })
})
