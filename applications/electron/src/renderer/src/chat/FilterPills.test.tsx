import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/react'
import { useState } from 'react'
import FilterPills from './FilterPills'
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

const checkboxFor = (pill: HTMLElement): HTMLInputElement => {
  const input = pill.querySelector('input[type="checkbox"]')
  if (input === null) throw new Error('filter-pill missing checkbox')
  return input as HTMLInputElement
}

describe('#1456 FilterPills — native HTML5 checkbox-in-label (Ryan-canon supersession)', () => {
  test('Scenario 1: exactly 3 pills render with canon labels chat/events/bpmux', () => {
    const { getByTestId, queryByTestId } = render(<Harness />)
    expect(getByTestId('filter-pill-chat')).toBeTruthy()
    expect(getByTestId('filter-pill-events')).toBeTruthy()
    expect(getByTestId('filter-pill-bpmux')).toBeTruthy()
    expect(queryByTestId('filter-pill-all')).toBeNull()
    expect(queryByTestId('filter-pill-insight')).toBeNull()
    expect(queryByTestId('filter-pill-activity')).toBeNull()
    expect(queryByTestId('filter-pill-system')).toBeNull()
  })

  test('Scenario 2: default chat ON; events + bpmux OFF (via native checkbox.checked)', () => {
    const { getByTestId } = render(<Harness />)
    expect(checkboxFor(getByTestId('filter-pill-chat')).checked).toBe(true)
    expect(checkboxFor(getByTestId('filter-pill-events')).checked).toBe(false)
    expect(checkboxFor(getByTestId('filter-pill-bpmux')).checked).toBe(false)
  })

  test('Scenario 3: clicking pill mute-toggles state (label-wraps-input implicit-association)', () => {
    const { getByTestId } = render(<Harness />)
    const chatPill = getByTestId('filter-pill-chat')
    expect(checkboxFor(chatPill).checked).toBe(true)
    fireEvent.click(chatPill)
    expect(checkboxFor(chatPill).checked).toBe(false)
    fireEvent.click(chatPill)
    expect(checkboxFor(chatPill).checked).toBe(true)
  })

  test('Scenario 4: all-off allowed; empty-hint shows when all muted', () => {
    const { getByTestId, queryByTestId } = render(<Harness />)
    expect(queryByTestId('filter-pills-empty-hint')).toBeNull()
    fireEvent.click(getByTestId('filter-pill-chat'))
    expect(getByTestId('filter-pills-empty-hint')).toBeTruthy()
  })

  test('Scenario 5: ON/OFF visual state distinguishable via data-state', () => {
    const { getByTestId } = render(<Harness />)
    expect(getByTestId('filter-pill-chat').getAttribute('data-state')).toBe('on')
    expect(getByTestId('filter-pill-events').getAttribute('data-state')).toBe('off')
  })

  test('Scenario 6: empty-data pill carries data-empty=true (count=0)', () => {
    const { getByTestId } = render(<Harness visibleCounts={{ chat: 5 }} />)
    expect(getByTestId('filter-pill-chat').getAttribute('data-empty')).toBeNull()
    expect(getByTestId('filter-pill-events').getAttribute('data-empty')).toBe('true')
    expect(getByTestId('filter-pill-bpmux').getAttribute('data-empty')).toBe('true')
  })

  test('Scenario 6 cont.: pill displays count badge from visible/hidden counts', () => {
    const { getByTestId } = render(
      <Harness visibleCounts={{ chat: 5 }} hiddenCounts={{ events: 12 }} />
    )
    expect(getByTestId('filter-pill-chat-count').textContent).toBe('(5)')
    expect(getByTestId('filter-pill-events-count').textContent).toBe('(12)')
    expect(getByTestId('filter-pill-bpmux-count').textContent).toBe('(0)')
  })

  test('Scenario 9: each pill input has role=checkbox + accessible aria-label', () => {
    const { getByTestId } = render(<Harness />)
    for (const cls of MESSAGE_CLASSES) {
      const pill = getByTestId(`filter-pill-${cls}`)
      const input = checkboxFor(pill)
      expect(input.getAttribute('type')).toBe('checkbox')
      const label = input.getAttribute('aria-label')
      expect(label).toMatch(new RegExp(`^${cls}`))
      expect(label).toMatch(/(shown|hidden)/)
    }
  })

  test('canon: zero <div> + zero <span> in rendered tree (feedback_html5_semantic_no_divs_spans)', () => {
    const { container } = render(<Harness />)
    expect(container.querySelectorAll('div').length).toBe(0)
    expect(container.querySelectorAll('span').length).toBe(0)
  })

  test('canon: no role overlay on a non-canonical element (feedback_first_rule_of_aria)', () => {
    const { container } = render(<Harness />)
    expect(container.querySelector('[role="switch"]')).toBeNull()
  })

  test('canon: fieldset group with legend carries accessible name', () => {
    const { container } = render(<Harness />)
    const fieldset = container.querySelector('fieldset.filter-pills-row')
    expect(fieldset).not.toBeNull()
    const legend = fieldset?.querySelector('legend')
    expect(legend?.textContent).toMatch(/message class/i)
  })
})
