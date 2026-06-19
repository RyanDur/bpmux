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

describe('#1456 FilterPills — native HTML5 checkbox-in-label (owner-canon supersession)', () => {
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
    expect(filterPills.emptyHintShown(/all categories muted/i)).toBe(false)
    await filterPills.toggle('chat')
    expect(filterPills.emptyHintShown(/all categories muted/i)).toBe(true)
  })

  test('Scenario 6: a zero count is labelled "0 messages"; a non-empty count is labelled its number', () => {
    render(<Harness visibleCounts={{ chat: 5 }} />)
    expect(filterPills.hasLabel(/chat messages.*\b5 messages/)).toBe(true)
    expect(filterPills.hasLabel(/events messages.*\b0 messages/)).toBe(true)
    expect(filterPills.hasLabel(/bpmux messages.*\b0 messages/)).toBe(true)
  })

  test('Scenario 6 cont.: the label carries the visible/hidden count', () => {
    render(<Harness visibleCounts={{ chat: 5 }} hiddenCounts={{ events: 12 }} />)
    expect(filterPills.hasLabel(/chat messages.*\b5 messages/)).toBe(true)
    expect(filterPills.hasLabel(/events messages.*\b12 messages/)).toBe(true)
    expect(filterPills.hasLabel(/bpmux messages.*\b0 messages/)).toBe(true)
  })

  test('Scenario 9: each pill is a checkbox labelled for its class + shown/hidden state', () => {
    render(<Harness />)
    for (const cls of MESSAGE_CLASSES) {
      expect(filterPills.hasLabel(new RegExp(`^${cls} messages (shown|hidden)`))).toBe(true)
    }
  })

  test('canon: fieldset group with legend carries accessible name', () => {
    render(<Harness />)
    expect(filterPills.group(/message class/i)).toBeTruthy()
  })
})
