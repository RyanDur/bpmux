import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, test } from 'vitest'
import BoardRoadmapSwitcher from './BoardRoadmapSwitcher'
import { boardRoadmapSwitcher as ui } from './BoardRoadmapSwitcher.page'

afterEach(() => cleanup())
beforeEach(() => {
  window.localStorage.clear()
})

const renderSwitcher = (): void => {
  render(
    <BoardRoadmapSwitcher
      boardLabel={
        <>
          <strong>Crux</strong>
          <small>Board</small>
          <i>●</i>
        </>
      }
      board={<p>board-content</p>}
      roadmap={<p>roadmap-content</p>}
    />
  )
}

describe('#1704 BoardRoadmapSwitcher — header-as-Board-tab sibling-switcher (design-of-record IA)', () => {
  test('renders the Board-tab (the header content) and the 🗺 Roadmap tab as the two siblings', () => {
    renderSwitcher()
    expect(ui.tabHasText('board', 'Crux')).toBe(true)
    expect(ui.tabText('board')).toContain('Board')
    expect(ui.tabText('roadmap')).toContain('🗺 Roadmap')
  })

  test('the sync-● is nested INSIDE the Board-tab label — part of the Board cluster, under its underline', () => {
    renderSwitcher()
    expect(ui.tabHasText('board', '●')).toBe(true)
  })

  test('defaults to Board — board panel mounted; the roadmap panel is NOT in the DOM (render-on-demand)', () => {
    renderSwitcher()
    expect(ui.isActive('board')).toBe(true)
    expect(ui.isActive('roadmap')).toBe(false)
    expect(screen.queryByText('board-content')).not.toBeNull()
    expect(screen.queryByText('roadmap-content')).toBeNull()
  })

  test('render-on-demand: only the active panel is in the DOM — board content present, roadmap mounts on toggle', async () => {
    renderSwitcher()
    expect(screen.queryByText('board-content')).not.toBeNull()
    expect(screen.queryByText('roadmap-content')).toBeNull()
    await ui.selectTab('roadmap')
    expect(screen.queryByText('roadmap-content')).not.toBeNull()
  })

  test('toggling to Roadmap mounts it; the board panel stays mounted so its state survives natively', async () => {
    renderSwitcher()
    await ui.selectTab('roadmap')
    expect(screen.queryByText('roadmap-content')).not.toBeNull()
    expect(screen.queryByText('board-content')).not.toBeNull()
  })
})

const NUDGE = {
  nextIterationNumber: 200,
  nextIterationName: 'Fornax',
  activeGapLabel: 'designer-review'
}

const renderWithNudge = (): void => {
  render(
    <BoardRoadmapSwitcher
      boardLabel={<small>Board</small>}
      board={<p>board-content</p>}
      roadmap={<p>roadmap-content</p>}
      nudge={NUDGE}
      currentIterationName="Lupus"
    />
  )
}

describe('#1705 close-approach readiness nudge — strip + tab-badge', () => {
  test('renders the close-approach strip (plain gap) + the ◐ tab-badge when a nudge is present', () => {
    renderWithNudge()
    const strip = ui.nudge()
    expect(strip.textContent).toContain('Lupus')
    expect(strip.textContent).toContain('Fornax')
    expect(strip.textContent).toContain('designer-review')
    expect(ui.attnShown()).toBe(true)
  })

  test('no nudge → neither the strip nor the tab-badge render (signal-honesty)', () => {
    renderSwitcher()
    expect(ui.isNudgeShown()).toBe(false)
    expect(ui.attnShown()).toBe(false)
  })

  test('dismissing the strip collapses it (data-dismissed) but the ◐ tab-badge survives (recognition-over-recall)', async () => {
    renderWithNudge()
    expect(ui.nudge().getAttribute('data-dismissed')).toBeNull()
    await ui.dismissNudge()
    expect(ui.nudge().getAttribute('data-dismissed')).toBe('true')
    expect(ui.attnShown()).toBe(true)
  })

  test('View roadmap → switches to the roadmap tab (a steer, not a gate) and hides the board-tab strip', async () => {
    renderWithNudge()
    await ui.viewRoadmapFromNudge()
    expect(ui.isActive('roadmap')).toBe(true)
    expect(ui.isNudgeShown()).toBe(false)
  })
})
