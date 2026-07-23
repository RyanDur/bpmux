import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import { useState } from 'react'
import Tabs, { type TabDef } from './Tabs'
import { tabs } from './Tabs.page'

afterEach(() => cleanup())

type Id = 'one' | 'two' | 'three'

const TABS: ReadonlyArray<TabDef<Id>> = [
  { id: 'one', label: 'One' },
  { id: 'two', label: 'Two' },
  { id: 'three', label: 'Three' }
]

const Harness = ({ initial = 'one' as Id }: { initial?: Id }): React.JSX.Element => {
  const [active, setActive] = useState<Id>(initial)
  return (
    <Tabs<Id>
      ariaLabel="Test tabs"
      tabs={TABS}
      activeId={active}
      onActiveChange={setActive}
      renderPanel={(id) => <output>{id}</output>}
    />
  )
}

describe('Tabs primitive — native HTML5 radio-group ', () => {
  test('renders fieldset (role=group) with 3 native radios + label-text', () => {
    render(<Harness />)
    expect(tabs.group('Test tabs')).toBeTruthy()
    expect(tabs.radioCount()).toBe(3)
    expect(tabs.everyRadioIsNativeInput()).toBe(true)
  })

  test('radios share the same name attribute (single radio-group)', () => {
    render(<Harness />)
    const groupName = tabs.groupName()
    expect(groupName).not.toBe('')
    expect(tabs.allShareName(groupName)).toBe(true)
  })

  test('active tab radio is checked; others are not', () => {
    render(<Harness initial="two" />)
    expect(tabs.radio(0).checked).toBe(false)
    expect(tabs.radio(1).checked).toBe(true)
    expect(tabs.radio(2).checked).toBe(false)
  })

  test('panels render as <article> with hidden on inactive panels', () => {
    render(<Harness />)
    expect(tabs.panelCount()).toBe(3)
    expect(tabs.panelHidden(0)).toBe(false)
    expect(tabs.panelHidden(1)).toBe(true)
    expect(tabs.panelHidden(2)).toBe(true)
  })

  test('Tabs.css renders explicit .tabs-panel[hidden] { display: none } rule (defensive substrate)', async () => {
    const { readFileSync } = await import('fs')
    const { resolve } = await import('path')
    const cssPath = resolve(__dirname, 'Tabs.css')
    const cssText = readFileSync(cssPath, 'utf-8')
    expect(cssText).toMatch(/\.tabs-panel\b[\s\S]*?&?\[hidden\][\s\S]*?display\s*:\s*none/m)
  })

  test('click on label activates the matching radio (label-wraps-radio implicit association)', async () => {
    render(<Harness />)
    expect(tabs.radio(0).checked).toBe(true)
    await tabs.clickTab('Two')
    expect(tabs.radio(0).checked).toBe(false)
    expect(tabs.radio(1).checked).toBe(true)
  })

  test('changing radio fires onActiveChange', async () => {
    render(<Harness />)
    await tabs.clickRadio(2)
    expect(tabs.radio(2).checked).toBe(true)
  })

  test('no ARIA role overlay on div (canon: feedback_first_rule_of_aria)', () => {
    render(<Harness />)
    expect(tabs.hasTablistRole()).toBe(false)
    expect(tabs.hasTabRole()).toBe(false)
    expect(tabs.hasTabpanelRole()).toBe(false)
  })

  test('legend carries the accessible name for the radio-group (visually hidden via CSS)', () => {
    render(<Harness />)
    expect(tabs.group('Test tabs')).toBeTruthy()
  })
})
