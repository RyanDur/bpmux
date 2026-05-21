import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, render, fireEvent } from '@testing-library/react'
import { useState } from 'react'
import Tabs, { type TabDef } from './Tabs'

afterEach(() => cleanup())

type Id = 'one' | 'two' | 'three'

const TABS: ReadonlyArray<TabDef<Id>> = [
  { id: 'one', label: 'One', tabTestId: 'tab-one', panelTestId: 'panel-one' },
  { id: 'two', label: 'Two', tabTestId: 'tab-two', panelTestId: 'panel-two' },
  { id: 'three', label: 'Three', tabTestId: 'tab-three', panelTestId: 'panel-three' }
]

const Harness = ({ initial = 'one' as Id }: { initial?: Id }): React.JSX.Element => {
  const [active, setActive] = useState<Id>(initial)
  return (
    <Tabs<Id>
      ariaLabel="Test tabs"
      tabs={TABS}
      activeId={active}
      onActiveChange={setActive}
      renderPanel={(id) => <span data-test={`panel-content-${id}`}>{id}</span>}
    />
  )
}

describe('#1453 Tabs primitive — ARIA + keyboard + click', () => {
  test('renders role=tablist with 3 role=tab buttons + aria-selected on active', () => {
    const { getByRole, getAllByRole } = render(<Harness />)
    const tablist = getByRole('tablist', { name: 'Test tabs' })
    expect(tablist).toBeTruthy()
    const tabs = getAllByRole('tab')
    expect(tabs.length).toBe(3)
    expect(tabs[0].getAttribute('aria-selected')).toBe('true')
    expect(tabs[1].getAttribute('aria-selected')).toBe('false')
    expect(tabs[2].getAttribute('aria-selected')).toBe('false')
  })

  test('active tab has tabIndex=0; inactive tabs have tabIndex=-1', () => {
    const { getAllByRole } = render(<Harness />)
    const tabs = getAllByRole('tab')
    expect(tabs[0].getAttribute('tabindex')).toBe('0')
    expect(tabs[1].getAttribute('tabindex')).toBe('-1')
    expect(tabs[2].getAttribute('tabindex')).toBe('-1')
  })

  test('panels have role=tabpanel + aria-labelledby + hidden on inactive', () => {
    const { getAllByRole } = render(<Harness />)
    const panels = getAllByRole('tabpanel', { hidden: true })
    expect(panels.length).toBe(3)
    expect(panels[0].hasAttribute('hidden')).toBe(false)
    expect(panels[1].hasAttribute('hidden')).toBe(true)
    expect(panels[2].hasAttribute('hidden')).toBe(true)
    expect(panels[0].getAttribute('aria-labelledby')).toBe('tab-one')
  })

  test('click on tab activates it', () => {
    const { getByTestId } = render(<Harness />)
    const two = getByTestId('tab-two')
    fireEvent.click(two)
    expect(two.getAttribute('aria-selected')).toBe('true')
    expect(getByTestId('tab-one').getAttribute('aria-selected')).toBe('false')
  })

  test('ArrowRight cycles forward; ArrowLeft cycles back; wraps around', () => {
    const { getByRole, getAllByRole } = render(<Harness />)
    const tablist = getByRole('tablist')
    fireEvent.keyDown(tablist, { key: 'ArrowRight' })
    expect(getAllByRole('tab')[1].getAttribute('aria-selected')).toBe('true')
    fireEvent.keyDown(tablist, { key: 'ArrowRight' })
    expect(getAllByRole('tab')[2].getAttribute('aria-selected')).toBe('true')
    fireEvent.keyDown(tablist, { key: 'ArrowRight' })
    expect(getAllByRole('tab')[0].getAttribute('aria-selected')).toBe('true')
    fireEvent.keyDown(tablist, { key: 'ArrowLeft' })
    expect(getAllByRole('tab')[2].getAttribute('aria-selected')).toBe('true')
  })

  test('Home jumps to first; End jumps to last', () => {
    const { getByRole, getAllByRole } = render(<Harness initial="two" />)
    const tablist = getByRole('tablist')
    fireEvent.keyDown(tablist, { key: 'End' })
    expect(getAllByRole('tab')[2].getAttribute('aria-selected')).toBe('true')
    fireEvent.keyDown(tablist, { key: 'Home' })
    expect(getAllByRole('tab')[0].getAttribute('aria-selected')).toBe('true')
  })

  test('Other keys do not change active tab', () => {
    const { getByRole, getAllByRole } = render(<Harness />)
    const tablist = getByRole('tablist')
    fireEvent.keyDown(tablist, { key: 'a' })
    fireEvent.keyDown(tablist, { key: 'Enter' })
    expect(getAllByRole('tab')[0].getAttribute('aria-selected')).toBe('true')
  })
})
