import { cleanup, render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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

  test('defaults to Board — board panel mounted + visible; the roadmap panel is NOT in the DOM (render-on-demand)', () => {
    const { getByTestId, queryByTestId } = renderSwitcher()
    const boardRadio = getByTestId('board-roadmap-tab-board').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    const roadmapRadio = getByTestId('board-roadmap-tab-roadmap').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    expect(boardRadio.checked).toBe(true)
    expect(roadmapRadio.checked).toBe(false)
    expect(getByTestId('board-roadmap-panel-board').hasAttribute('hidden')).toBe(false)
    expect(queryByTestId('board-roadmap-panel-roadmap')).toBeNull()
  })

  test('render-on-demand: only the active panel is in the DOM — board content present, roadmap mounts on toggle', async () => {
    const { getByTestId, queryByTestId } = renderSwitcher()
    expect(
      getByTestId('board-roadmap-panel-board').querySelector('[data-test="board-marker"]')
    ).toBeTruthy()
    expect(queryByTestId('board-roadmap-panel-roadmap')).toBeNull()
    await userEvent.click(getByTestId('board-roadmap-tab-roadmap'))
    expect(
      getByTestId('board-roadmap-panel-roadmap').querySelector('[data-test="roadmap-marker"]')
    ).toBeTruthy()
  })

  test('toggling to Roadmap mounts it and HIDES the board panel — the board panel stays mounted so its state survives natively', async () => {
    const { getByTestId } = renderSwitcher()
    await userEvent.click(getByTestId('board-roadmap-tab-roadmap'))
    expect(getByTestId('board-roadmap-panel-roadmap').hasAttribute('hidden')).toBe(false)
    const boardPanel = getByTestId('board-roadmap-panel-board')
    expect(boardPanel.hasAttribute('hidden')).toBe(true)
    expect(boardPanel.querySelector('[data-test="board-marker"]')).toBeTruthy()
  })
})

const NUDGE = {
  nextIterationNumber: 200,
  nextIterationName: 'Fornax',
  activeGapLabel: 'designer-review'
}

const renderWithNudge = (): ReturnType<typeof render> =>
  render(
    <BoardRoadmapSwitcher
      boardLabel={<small>Board</small>}
      board={<p data-test="board-marker">board-content</p>}
      roadmap={<p data-test="roadmap-marker">roadmap-content</p>}
      nudge={NUDGE}
      currentIterationName="Lupus"
    />
  )

describe('#1705 close-approach readiness nudge — strip + tab-badge', () => {
  test('renders the close-approach strip (plain gap) + the ◐ tab-badge when a nudge is present', () => {
    const { getByTestId } = renderWithNudge()
    const strip = getByTestId('close-approach-nudge')
    expect(strip.textContent).toContain('Lupus')
    expect(strip.textContent).toContain('Fornax')
    expect(strip.textContent).toContain('designer-review')
    expect(getByTestId('board-roadmap-attn')).toBeTruthy()
  })

  test('no nudge → neither the strip nor the tab-badge render (signal-honesty)', () => {
    const { queryByTestId } = renderSwitcher()
    expect(queryByTestId('close-approach-nudge')).toBeNull()
    expect(queryByTestId('board-roadmap-attn')).toBeNull()
  })

  test('dismissing the strip collapses it (data-dismissed) but the ◐ tab-badge survives (recognition-over-recall)', async () => {
    const { getByTestId } = renderWithNudge()
    expect(getByTestId('close-approach-nudge').getAttribute('data-dismissed')).toBeNull()
    await userEvent.click(getByTestId('close-approach-nudge-dismiss'))
    expect(getByTestId('close-approach-nudge').getAttribute('data-dismissed')).toBe('true')
    expect(getByTestId('board-roadmap-attn')).toBeTruthy()
  })

  test('View roadmap → switches to the roadmap tab (a steer, not a gate) and hides the board-tab strip', async () => {
    const { getByTestId } = renderWithNudge()
    await userEvent.click(getByTestId('close-approach-nudge-view'))
    const roadmapRadio = getByTestId('board-roadmap-tab-roadmap').querySelector(
      'input[type="radio"]'
    ) as HTMLInputElement
    expect(roadmapRadio.checked).toBe(true)
    expect(getByTestId('close-approach-nudge').hasAttribute('hidden')).toBe(true)
  })
})
