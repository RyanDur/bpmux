import { afterEach, describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import Iteration from './Iteration'
import { iterationLabel } from './labelSchema'
import { boardIteration, seedIterations } from './Iteration.fixtures'
import { resetStateForTest } from '../../state/store'
import { startSentFrameCapture, stopSentFrameCapture } from '../../state/testCapture'
import type { Story } from '../types'
afterEach(() => {
  cleanup()
  stopSentFrameCapture()
  resetStateForTest()
})

const makeDataTransfer = (): DataTransfer => {
  const store: Record<string, string> = {}
  const types: string[] = []
  return {
    setData: (type: string, value: string) => {
      store[type] = value
      if (!types.includes(type)) types.push(type)
    },
    getData: (type: string) => store[type] ?? '',
    setDragImage: () => {},
    get types() {
      return types
    },
    effectAllowed: 'move',
    dropEffect: 'move'
  } as unknown as DataTransfer
}

const memberStory = (overrides: Partial<Story>): Story => ({
  id: 9100,
  title: 'member',
  state: 'icebox',
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const renderIteration = (members: Story[] = []): string =>
  renderToStaticMarkup(<Iteration stories={members} onOpen={() => {}} />)

describe('Iteration — renders regardless of member-count or open/closed state', () => {
  test(': open + 0 members surfaces in icebox-iterations-list', () => {
    seedIterations([boardIteration({ id: 9000, title: 'Iteration Lyra' })])
    const html = renderIteration()
    expect(html).toContain('data-test="board-iteration-9000"')
    expect(html).toContain('Iteration Lyra')
    expect(html).toContain('(0)')
  })

  test('(): closed + 0 members surfaces with closed badge', () => {
    seedIterations([boardIteration({ id: 9001, title: 'Iteration WalkVerify', complete: true })])
    const html = renderIteration()
    expect(html).toContain('data-test="board-iteration-9001"')
    expect(html).toContain('Iteration WalkVerify')
    expect(html).toContain('(closed)')
    expect(html).toContain('(0)')
  })

  test('(): open + N members surfaces with member-count', () => {
    seedIterations([
      boardIteration({
        id: 9002,
        title: 'Iteration Carina',
        stories: [9201, 9202]
      })
    ])
    const m1 = memberStory({ id: 9201 })
    const m2 = memberStory({ id: 9202 })
    const html = renderIteration([m1, m2])
    expect(html).toContain('data-test="board-iteration-9002"')
    expect(html).toContain('Iteration Carina')
    expect(html).toContain('(2)')
    expect(html).not.toContain('(closed)')
  })

  test('(): closed + N members surfaces with closed badge + count', () => {
    seedIterations([
      boardIteration({
        id: 9003,
        title: 'Iteration Andromeda',
        complete: true,
        stories: [9301]
      })
    ])
    const m1 = memberStory({ id: 9301 })
    const html = renderIteration([m1])
    expect(html).toContain('data-test="board-iteration-9003"')
    expect(html).toContain('Iteration Andromeda')
    expect(html).toContain('(closed)')
    expect(html).toContain('(1)')
  })

  test('returns null when no iterations and no loose stories (existing behavior preserved)', () => {
    expect(renderIteration()).toBe('')
  })
})

describe('Iteration — member count/membership derives from the iteration container (iteration.stories)', () => {
  test(': container members surface in count', () => {
    seedIterations([
      boardIteration({
        id: 1464,
        title: 'Iteration Reticulum',
        stories: [9501, 9502]
      })
    ])
    const m1 = memberStory({ id: 9501 })
    const m2 = memberStory({ id: 9502 })
    const html = renderIteration([m1, m2])
    expect(html).toContain('data-test="board-iteration-1464"')
    expect(html).toContain('Iteration Reticulum')
    expect(html).toContain('(2)')
  })

  test('(): legacy-label-only member is dropped; only container members count', () => {
    seedIterations([
      boardIteration({
        id: 1491,
        title: 'Iteration Caelum',
        stories: [9602, 9603]
      })
    ])
    const legacyOnly = memberStory({
      id: 9601,
      labels: ['Feature', iterationLabel('Caelum', 100)]
    })
    const canonicalOnly = memberStory({ id: 9602 })
    const both = memberStory({
      id: 9603,
      labels: ['Feature', iterationLabel('Caelum', 300)]
    })
    const html = renderIteration([legacyOnly, canonicalOnly, both])
    expect(html).toContain('data-test="board-iteration-1491"')
    expect(html).toContain('Iteration Caelum')
    expect(html).toContain('(2)')
  })

  test('(drill-in-axis): container members enumerated in member-list', () => {
    seedIterations([
      boardIteration({
        id: 1398,
        title: 'Iteration Microscopium',
        stories: [9701]
      })
    ])
    const m1 = memberStory({
      id: 9701,
      title: 'canonical-member-one'
    })
    const html = renderIteration([m1])
    expect(html).toContain('canonical-member-one')
  })

  test('(other-iteration-isolation): membership for iter A does NOT count toward iter B', () => {
    seedIterations([
      boardIteration({ id: 1464, title: 'Iteration Reticulum', stories: [] }),
      boardIteration({
        id: 1491,
        title: 'Iteration Caelum',
        stories: [9801]
      })
    ])
    const memberOfCaelum = memberStory({ id: 9801 })
    const html = renderIteration([memberOfCaelum])
    const reticulumOpenTag =
      '<details class="board-iteration-card" data-test="board-iteration-1464"'
    const caelumOpenTag = '<details class="board-iteration-card" data-test="board-iteration-1491"'
    const reticulumIdx = html.indexOf(reticulumOpenTag)
    const caelumIdx = html.indexOf(caelumOpenTag)
    expect(reticulumIdx).toBeGreaterThanOrEqual(0)
    expect(caelumIdx).toBeGreaterThanOrEqual(0)
    const reticulumChunk = html.slice(reticulumIdx, caelumIdx)
    const caelumChunk = html.slice(caelumIdx)
    expect(reticulumChunk).toContain('(0)')
    expect(caelumChunk).toContain('(1)')
  })
})

describe('facet 1 — member workflow-state is legible in the iteration-view', () => {
  test('a ready member renders a visible state label hooked on data-state', () => {
    seedIterations([boardIteration({ id: 1543, title: 'Iteration Crux', stories: [1537] })])
    const readyMember = memberStory({
      id: 1537,
      title: 'ready-member',
      state: 'ready'
    })
    const html = renderIteration([readyMember])
    expect(html).toContain('ready-member')
    expect(html).toContain('board-iteration-member-state')
    expect(html).toContain('data-state="ready"')
    expect(html).toContain('>Ready<')
  })

  test('in-progress and desk-check members carry their own legible state labels', () => {
    seedIterations([
      boardIteration({
        id: 1543,
        title: 'Iteration Crux',
        stories: [1538, 1539]
      })
    ])
    const inProgress = memberStory({
      id: 1538,
      title: 'in-progress-member',
      state: 'in-progress'
    })
    const deskCheck = memberStory({
      id: 1539,
      title: 'desk-check-member',
      state: 'desk-check'
    })
    const html = renderIteration([inProgress, deskCheck])
    expect(html).toContain('data-state="in-progress"')
    expect(html).toContain('>In Progress<')
    expect(html).toContain('data-state="desk-check"')
    expect(html).toContain('>Desk Check<')
  })

  test('an icebox member stays neutral — no active-state label (icebox is the planned default)', () => {
    seedIterations([boardIteration({ id: 1543, title: 'Iteration Crux', stories: [1540] })])
    const iceboxMember = memberStory({
      id: 1540,
      title: 'icebox-member',
      state: 'icebox'
    })
    const html = renderIteration([iceboxMember])
    expect(html).toContain('icebox-member')
    expect(html).not.toContain('board-iteration-member-state')
  })
})

describe('Iteration list scaling — section-grouped (Active+Planned / Closed)', () => {
  test('renders 2 sub-sections with canonical labels + counts', () => {
    seedIterations([
      boardIteration({ id: 9001, title: 'Iteration ActiveA' }),
      boardIteration({ id: 9002, title: 'Iteration ActiveB' }),
      boardIteration({ id: 9003, title: 'Iteration ClosedA', complete: true })
    ])
    const html = renderIteration()
    expect(html).toContain('data-test="board-iterations-section-active-planned"')
    expect(html).toContain('data-test="board-iterations-section-closed"')
    expect(html).toContain('Active + Planned')
    expect(html).toContain('Closed')
    const activeCountIdx = html.indexOf('board-iterations-section-active-planned-count')
    const closedCountIdx = html.indexOf('board-iterations-section-closed-count')
    expect(html.slice(activeCountIdx, activeCountIdx + 200)).toContain('(2)')
    expect(html.slice(closedCountIdx, closedCountIdx + 200)).toContain('(1)')
  })

  test('Active+Planned section open by default; Closed collapsed', () => {
    seedIterations([
      boardIteration({ id: 9001, title: 'Iteration A' }),
      boardIteration({ id: 9002, title: 'Iteration B', complete: true })
    ])
    const html = renderIteration()
    const activeMatch = html.match(
      /data-test="board-iterations-section-active-planned"[^>]*open[^>]*>/
    )
    const closedMatch = html.match(/data-test="board-iterations-section-closed"([^>]*)>/)
    expect(activeMatch).toBeTruthy()
    expect(closedMatch?.[1]).not.toContain(' open')
  })

  test('each section count reflects partition counts', () => {
    seedIterations([
      boardIteration({ id: 9001, title: 'A' }),
      boardIteration({ id: 9002, title: 'B' }),
      boardIteration({ id: 9003, title: 'C' }),
      boardIteration({ id: 9101, title: 'X', complete: true }),
      boardIteration({ id: 9102, title: 'Y', complete: true })
    ])
    const html = renderIteration()
    const activeCountIdx = html.indexOf('board-iterations-section-active-planned-count')
    const closedCountIdx = html.indexOf('board-iterations-section-closed-count')
    expect(html.slice(activeCountIdx, activeCountIdx + 200)).toContain('(3)')
    expect(html.slice(closedCountIdx, closedCountIdx + 200)).toContain('(2)')
  })

  test(': no iteration is anchored when none is designated (honest-empty, no auto-pick)', () => {
    seedIterations([
      boardIteration({ id: 9001, title: 'Iteration First', iteration_order: 100 }),
      boardIteration({ id: 9002, title: 'Iteration Second', iteration_order: 200 })
    ])
    const html = renderIteration()
    expect(html.match(/data-current-iteration="true"/)).toBeNull()
  })

  test(': the explicitly-designated iteration is anchored regardless of order or in-flight children', () => {
    seedIterations([
      boardIteration({
        id: 9001,
        title: 'Iteration First',
        iteration_order: 100,
        stories: [9501]
      }),
      boardIteration({
        id: 9002,
        title: 'Iteration Second',
        iteration_order: 200,
        is_current: true,
        stories: [9502]
      })
    ])
    const firstIceboxChild = memberStory({
      id: 9501,
      state: 'icebox'
    })
    const secondInFlightChild = memberStory({
      id: 9502,
      state: 'in-progress'
    })
    const html = renderIteration([firstIceboxChild, secondInFlightChild])
    expect(
      html.match(/data-test="board-iteration-9002"[^>]*data-current-iteration="true"/)
    ).toBeTruthy()
    expect(
      html.match(/data-test="board-iteration-9001"[^>]*data-current-iteration="true"/)
    ).toBeNull()
  })

  test('Scenario regression: closed iterations omit drag-handle (grip)', () => {
    seedIterations([boardIteration({ id: 9001, title: 'Iteration ClosedA', complete: true })])
    const html = renderIteration()
    const closedCardChunk = html.slice(html.indexOf('data-test="board-iteration-9001"'))
    expect(closedCardChunk).not.toContain('data-test="iteration-grip"')
  })
})

describe('iteration-row redesign — summary toggle-only + grip-sibling-right + navigate-in-body', () => {
  const summaryOf = (html: string): string => {
    const start = html.indexOf('<summary class="board-iteration-summary"')
    const end = html.indexOf('</summary>', start)
    return html.slice(start, end)
  }

  test('summary is toggle-only — no nested interactive element (grip + navigate moved out)', () => {
    seedIterations([boardIteration({ id: 9000, title: 'Iteration Lyra' })])
    expect(summaryOf(renderIteration())).not.toContain('<button')
  })

  test('grip is a SIBLING of <details> (outside the summary), within the iteration-row parent', () => {
    seedIterations([boardIteration({ id: 9000, title: 'Iteration Lyra' })])
    const html = renderIteration()
    expect(html).toContain('class="board-iteration-row"')
    expect(summaryOf(html)).not.toContain('data-test="iteration-grip"')
    const gripIdx = html.indexOf('data-test="iteration-grip"')
    const detailsCloseIdx = html.indexOf('</details>')
    expect(gripIdx).toBeGreaterThan(detailsCloseIdx)
  })

  test('navigate (view iteration details) lives in the body, not the summary', () => {
    seedIterations([boardIteration({ id: 9000, title: 'Iteration Lyra' })])
    const html = renderIteration()
    expect(html).toContain('view iteration details')
    expect(summaryOf(html)).not.toContain('view iteration details')
  })

  test('grip is a keyboard-operable reorder control (tabindex=0 + aria-keyshortcuts)', () => {
    seedIterations([boardIteration({ id: 9000, title: 'Iteration Lyra' })])
    const html = renderIteration()
    const gripIdx = html.indexOf('data-test="iteration-grip"')
    const gripTagStart = html.lastIndexOf('<button', gripIdx)
    const gripTag = html.slice(gripTagStart, html.indexOf('>', gripIdx))
    expect(gripTag).toContain('tabindex="0"')
    expect(gripTag).toContain('aria-keyshortcuts="Control+Shift+ArrowUp Control+Shift+ArrowDown"')
  })
})

describe('iteration-row drag-sort dispatches a reorder over the wire (truth-on-the-wire)', () => {
  test('dropping a dragged iteration onto a sibling sends its new 1-indexed planning position', () => {
    seedIterations([
      boardIteration({ id: 9001, title: 'Iteration A', iteration_order: 100 }),
      boardIteration({ id: 9002, title: 'Iteration B', iteration_order: 200 })
    ])
    render(<Iteration stories={[]} onOpen={() => {}} />)

    const sent = startSentFrameCapture()
    const grips = screen.getAllByTestId('iteration-grip')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grips[0], { dataTransfer })
    const rows = document.querySelectorAll('.board-iteration-row')
    fireEvent.dragOver(rows[1] as HTMLElement, { dataTransfer })
    fireEvent.drop(rows[1] as HTMLElement, { dataTransfer })

    expect(sent).toEqual([{ type: 'reorder_iteration', iteration_id: 9001, position: 2 }])
  })

  test('an unordered iteration (how create_iteration makes them) still dispatches a position', () => {
    seedIterations([
      boardIteration({ id: 9001, title: 'Iteration A' }),
      boardIteration({ id: 9002, title: 'Iteration B' })
    ])
    render(<Iteration stories={[]} onOpen={() => {}} />)

    const sent = startSentFrameCapture()
    const grips = screen.getAllByTestId('iteration-grip')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grips[1], { dataTransfer })
    const rows = document.querySelectorAll('.board-iteration-row')
    fireEvent.dragOver(rows[0] as HTMLElement, { dataTransfer })
    fireEvent.drop(rows[0] as HTMLElement, { dataTransfer })

    expect(sent).toEqual([{ type: 'reorder_iteration', iteration_id: 9002, position: 1 }])
  })

  test('closed iterations are not drag sources (no grip) — drag-sort is active-planned only', () => {
    seedIterations([
      boardIteration({ id: 9003, title: 'Open Iteration', iteration_order: 100 }),
      boardIteration({
        id: 9004,
        title: 'Closed Iteration',
        iteration_order: 200,
        complete: true
      })
    ])
    render(<Iteration stories={[]} onOpen={() => {}} />)
    expect(screen.getAllByTestId('iteration-grip')).toHaveLength(1)
  })
})
