import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import { useState } from 'react'
import FilterPills from './FilterPills'
import { filterPills } from './FilterPills.page'
import { MESSAGE_CLASSES, type MessageClass, type MessageFilter } from '../hooks/useMessageFilter'

afterEach(() => cleanup())
beforeEach(() => window.localStorage.clear())

const Harness = ({
  initial = ['chat'],
  visibleCounts = {},
  hiddenCounts = {}
}: {
  initial?: MessageClass[]
  visibleCounts?: Partial<Record<MessageClass, number>>
  hiddenCounts?: Partial<Record<MessageClass, number>>
}): React.JSX.Element => {
  const [visible, setVisible] = useState<Set<MessageClass>>(new Set(initial))
  const filter: MessageFilter = {
    visible,
    toggle: (cls) =>
      setVisible((prev) => {
        const next = new Set(prev)
        if (next.has(cls)) next.delete(cls)
        else next.add(cls)
        return next
      }),
    reset: () => setVisible(new Set(['chat'])),
    setOnly: (cls) => setVisible(new Set([cls]))
  }
  return <FilterPills filter={filter} hiddenCounts={hiddenCounts} visibleCounts={visibleCounts} />
}

describe('#1456 FilterPills — native HTML5 checkbox-in-label (Ryan-canon supersession)', () => {
  test('Scenario 1: exactly 3 pills render with canon labels chat/events/bpmux', () => {
    render(<Harness />)
    expect(filterPills.isRendered('chat')).toBe(true)
    expect(filterPills.isRendered('events')).toBe(true)
    expect(filterPills.isRendered('bpmux')).toBe(true)
    expect(filterPills.isRendered('all')).toBe(false)
    expect(filterPills.isRendered('insight')).toBe(false)
    expect(filterPills.isRendered('activity')).toBe(false)
    expect(filterPills.isRendered('system')).toBe(false)
  })

  test('Scenario 2: default chat ON; events + bpmux OFF (via native checkbox.checked)', () => {
    render(<Harness />)
    expect(filterPills.isOn('chat')).toBe(true)
    expect(filterPills.isOn('events')).toBe(false)
    expect(filterPills.isOn('bpmux')).toBe(false)
  })

  test('Scenario 3: clicking pill mute-toggles state (label-wraps-input implicit-association)', async () => {
    render(<Harness />)
    expect(filterPills.isOn('chat')).toBe(true)
    await filterPills.toggle('chat')
    expect(filterPills.isOn('chat')).toBe(false)
    await filterPills.toggle('chat')
    expect(filterPills.isOn('chat')).toBe(true)
  })

  test('Scenario 4: all-off allowed; empty-hint shows when all muted', async () => {
    render(<Harness />)
    expect(filterPills.emptyHintShown()).toBe(false)
    await filterPills.toggle('chat')
    expect(filterPills.emptyHintShown()).toBe(true)
  })

  test('Scenario 5: ON/OFF visual state distinguishable via data-state', () => {
    render(<Harness />)
    expect(filterPills.state('chat')).toBe('on')
    expect(filterPills.state('events')).toBe('off')
  })

  test('Scenario 6: empty-data pill carries data-empty=true (count=0)', () => {
    render(<Harness visibleCounts={{ chat: 5 }} />)
    expect(filterPills.isMarkedEmpty('chat')).toBe(false)
    expect(filterPills.isMarkedEmpty('events')).toBe(true)
    expect(filterPills.isMarkedEmpty('bpmux')).toBe(true)
  })

  test('Scenario 6 cont.: pill displays count badge from visible/hidden counts', () => {
    render(<Harness visibleCounts={{ chat: 5 }} hiddenCounts={{ events: 12 }} />)
    expect(filterPills.count('chat')).toBe('(5)')
    expect(filterPills.count('events')).toBe('(12)')
    expect(filterPills.count('bpmux')).toBe('(0)')
  })

  test('Scenario 9: each pill input has role=checkbox + accessible aria-label', () => {
    render(<Harness />)
    for (const cls of MESSAGE_CLASSES) {
      const label = filterPills.checkboxLabel(cls)
      expect(label).toMatch(new RegExp(`^${cls}`))
      expect(label).toMatch(/(shown|hidden)/)
    }
  })

  test('canon: zero <div> + zero <span> in rendered tree (feedback_html5_semantic_no_divs_spans)', () => {
    render(<Harness />)
    expect(filterPills.divCount()).toBe(0)
    expect(filterPills.spanCount()).toBe(0)
  })

  test('canon: no role overlay on a non-canonical element (feedback_first_rule_of_aria)', () => {
    render(<Harness />)
    expect(filterPills.hasSwitchRole()).toBe(false)
  })

  test('canon: fieldset group with legend carries accessible name', () => {
    render(<Harness />)
    expect(filterPills.groupLegend()).toMatch(/message class/i)
  })
})
