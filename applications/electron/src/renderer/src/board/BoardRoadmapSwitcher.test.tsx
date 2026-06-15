import { cleanup, fireEvent, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import BoardRoadmapSwitcher from './BoardRoadmapSwitcher'

afterEach(() => cleanup())
beforeEach(() => {
  window.localStorage.clear()
})

const renderSwitcher = (): ReturnType<typeof render> =>
  render(
    <BoardRoadmapSwitcher
      boardLabel={
        <>
          <strong data-test="iter">Crux</strong>
          <small>Board</small>
          <i data-test="sync-marker">●</i>
        </>
      }
      board={<p data-test="board-marker">board-content</p>}
      roadmap={<p data-test="roadmap-marker">roadmap-content</p>}
    />
  )

describe('#1704 BoardRoadmapSwitcher — header-as-Board-tab sibling-switcher (design-of-record IA)', () => {
  test('renders the Board-tab (the header content) and the 🗺 Roadmap tab as the two siblings', () => {
    const { getByTestId } = renderSwitcher()
    const boardTab = getByTestId('board-roadmap-tab-board')
    expect(boardTab.querySelector('[data-test="iter"]')?.textContent).toBe('Crux')
    expect(boardTab.textContent).toContain('Board')
    expect(getByTestId('board-roadmap-tab-roadmap').textContent).toContain('🗺 Roadmap')
  })

  test('the sync-● is nested INSIDE the Board-tab label — part of the Board cluster, under its underline', () => {
    const { getByTestId } = renderSwitcher()
    const syncMarker = getByTestId('sync-marker')
    expect(syncMarker).toBeTruthy()
    expect(getByTestId('board-roadmap-tab-board').contains(syncMarker)).toBe(true)
  })

  test('defaults to Board — board panel visible, roadmap hidden', () => {
    const { getByTestId } = renderSwitcher()
    const boardRadio = getByTestId('board-roadmap-tab-board').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    const roadmapRadio = getByTestId('board-roadmap-tab-roadmap').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    expect(boardRadio.checked).toBe(true)
    expect(roadmapRadio.checked).toBe(false)
    expect(getByTestId('board-roadmap-panel-board').hasAttribute('hidden')).toBe(false)
    expect(getByTestId('board-roadmap-panel-roadmap').hasAttribute('hidden')).toBe(true)
  })

  test('Board panel mounts the board content; Roadmap panel mounts the roadmap content', () => {
    const { getByTestId } = renderSwitcher()
    expect(
      getByTestId('board-roadmap-panel-board').querySelector('[data-test="board-marker"]')
    ).toBeTruthy()
    expect(
      getByTestId('board-roadmap-panel-roadmap').querySelector('[data-test="roadmap-marker"]')
    ).toBeTruthy()
  })

  test('clicking 🗺 Roadmap activates roadmap; Board becomes inactive', () => {
    const { getByTestId } = renderSwitcher()
    fireEvent.click(getByTestId('board-roadmap-tab-roadmap'))
    expect(getByTestId('board-roadmap-panel-roadmap').hasAttribute('hidden')).toBe(false)
    expect(getByTestId('board-roadmap-panel-board').hasAttribute('hidden')).toBe(true)
  })
})
