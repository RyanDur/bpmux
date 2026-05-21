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
      renderPanel={(id) => <output data-test={`panel-content-${id}`}>{id}</output>}
    />
  )
}

describe('#1453 Tabs primitive — native HTML5 radio-group (Ryan-canon supersession)', () => {
  test('renders fieldset (role=group) with 3 native radios + label-text', () => {
    const { getByRole, getAllByRole } = render(<Harness />)
    const group = getByRole('group', { name: 'Test tabs' })
    expect(group).toBeTruthy()
    const radios = getAllByRole('radio')
    expect(radios.length).toBe(3)
    expect(radios.every((r) => r.getAttribute('type') === 'radio')).toBe(true)
    expect(radios.every((r) => r.tagName.toLowerCase() === 'input')).toBe(true)
  })

  test('radios share the same name attribute (single radio-group)', () => {
    const { getAllByRole } = render(<Harness />)
    const radios = getAllByRole('radio') as HTMLInputElement[]
    const groupName = radios[0].name
    expect(groupName).not.toBe('')
    expect(radios.every((r) => r.name === groupName)).toBe(true)
  })

  test('active tab radio is checked; others are not', () => {
    const { getAllByRole } = render(<Harness initial="two" />)
    const radios = getAllByRole('radio') as HTMLInputElement[]
    expect(radios[0].checked).toBe(false)
    expect(radios[1].checked).toBe(true)
    expect(radios[2].checked).toBe(false)
  })

  test('panels render as <article> with hidden on inactive panels', () => {
    const { container } = render(<Harness />)
    const articles = container.querySelectorAll('article.tabs-panel')
    expect(articles.length).toBe(3)
    expect((articles[0] as HTMLElement).hidden).toBe(false)
    expect((articles[1] as HTMLElement).hidden).toBe(true)
    expect((articles[2] as HTMLElement).hidden).toBe(true)
  })

  test('Tabs.css renders explicit .tabs-panel[hidden] { display: none } rule (defensive substrate)', async () => {
    const { readFileSync } = await import('fs')
    const { resolve } = await import('path')
    const cssPath = resolve(__dirname, 'Tabs.css')
    const cssText = readFileSync(cssPath, 'utf-8')
    expect(cssText).toMatch(/\.tabs-panel\b[\s\S]*?&?\[hidden\][\s\S]*?display\s*:\s*none/m)
  })

  test('click on label activates the matching radio (label-wraps-radio implicit association)', () => {
    const { getByTestId, getAllByRole } = render(<Harness />)
    const radios = getAllByRole('radio') as HTMLInputElement[]
    expect(radios[0].checked).toBe(true)
    fireEvent.click(getByTestId('tab-two'))
    expect(radios[0].checked).toBe(false)
    expect(radios[1].checked).toBe(true)
  })

  test('changing radio fires onActiveChange', () => {
    const { getAllByRole } = render(<Harness />)
    const radios = getAllByRole('radio') as HTMLInputElement[]
    fireEvent.click(radios[2])
    expect(radios[2].checked).toBe(true)
  })

  test('no <div> or <span> in rendered tree (canon: feedback_html5_semantic_no_divs_spans)', () => {
    const { container } = render(<Harness />)
    expect(container.querySelectorAll('div').length).toBe(0)
    expect(container.querySelectorAll('span').length).toBe(0)
  })

  test('no ARIA role overlay on div (canon: feedback_first_rule_of_aria)', () => {
    const { queryByRole, queryAllByRole } = render(<Harness />)
    expect(queryByRole('tablist')).toBeNull()
    expect(queryAllByRole('tab')).toEqual([])
    expect(queryAllByRole('tabpanel', { hidden: true })).toEqual([])
  })

  test('legend carries the accessible name for the radio-group (visually hidden via CSS)', () => {
    const { container } = render(<Harness />)
    const legend = container.querySelector('legend')
    expect(legend).not.toBeNull()
    expect(legend?.textContent).toBe('Test tabs')
  })
})
