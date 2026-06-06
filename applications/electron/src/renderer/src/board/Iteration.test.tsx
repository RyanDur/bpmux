import { afterEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import Iteration from './Iteration'
import { iterationLabel, iterationOrderLabel } from './labelSchema'
import type { BoardStory } from './types'

const { postLabelMutationMock } = vi.hoisted(() => ({ postLabelMutationMock: vi.fn() }))

vi.mock('./iterationMembers', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./iterationMembers')>()
  return { ...actual, postLabelMutation: postLabelMutationMock }
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

const iterationStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9000,
  title: 'Iteration Lyra',
  state: 'icebox',
  gh_state: 'open',
  labels: ['Iteration'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const activeIterationNumbers = (): number[] => {
  const section = document.querySelector('[data-test="board-iterations-section-active-planned"]')
  if (section === null) return []
  return Array.from(section.querySelectorAll('.board-iteration-card')).map((el) =>
    Number((el.getAttribute('data-test') ?? '').replace('board-iteration-', ''))
  )
}

const memberStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9100,
  title: 'member',
  state: 'icebox',
  gh_state: 'open',
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1413 Iteration — renders regardless of member-count or open/closed state', () => {
  test('Scenario 1 (fix-anchor): open + 0 members surfaces in icebox-iterations-list', () => {
    const lyra = iterationStory({ number: 9000, title: 'Iteration Lyra', gh_state: 'open' })
    const html = renderToStaticMarkup(<Iteration stories={[lyra]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9000"')
    expect(html).toContain('Iteration Lyra')
    expect(html).toContain('(0)')
  })

  test('Scenario 2 (regression-anchor): closed + 0 members surfaces with closed badge', () => {
    const walkVerify = iterationStory({
      number: 9001,
      title: 'Iteration WalkVerify',
      gh_state: 'closed'
    })
    const html = renderToStaticMarkup(<Iteration stories={[walkVerify]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9001"')
    expect(html).toContain('Iteration WalkVerify')
    expect(html).toContain('(closed)')
    expect(html).toContain('(0)')
  })

  test('Scenario 3 (regression-anchor): open + N members surfaces with member-count', () => {
    const carina = iterationStory({ number: 9002, title: 'Iteration Carina', gh_state: 'open' })
    const m1 = memberStory({
      number: 9201,
      iteration_membership: { iteration_number: 9002, position: 100 }
    })
    const m2 = memberStory({
      number: 9202,
      iteration_membership: { iteration_number: 9002, position: 200 }
    })
    const html = renderToStaticMarkup(<Iteration stories={[carina, m1, m2]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9002"')
    expect(html).toContain('Iteration Carina')
    expect(html).toContain('(2)')
    expect(html).not.toContain('(closed)')
  })

  test('Scenario 4 (regression-anchor): closed + N members surfaces with closed badge + count', () => {
    const andromeda = iterationStory({
      number: 9003,
      title: 'Iteration Andromeda',
      gh_state: 'closed'
    })
    const m1 = memberStory({
      number: 9301,
      iteration_membership: { iteration_number: 9003, position: 100 }
    })
    const html = renderToStaticMarkup(<Iteration stories={[andromeda, m1]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9003"')
    expect(html).toContain('Iteration Andromeda')
    expect(html).toContain('(closed)')
    expect(html).toContain('(1)')
  })

  test('returns null when no iterations and no loose stories (existing behavior preserved)', () => {
    const html = renderToStaticMarkup(<Iteration stories={[]} onOpen={() => {}} />)
    expect(html).toBe('')
  })
})

describe('#1492 Iteration — reads canonical-relation iteration_membership typed field (sister to #1399 substrate-data, post-fail-desk-check rework via json.rs typed wire-field)', () => {
  test('Scenario 5 (fix-anchor): canonical-relation members via typed field surface in count', () => {
    const reticulum = iterationStory({
      number: 1464,
      title: 'Iteration Reticulum',
      gh_state: 'open'
    })
    const m1 = memberStory({
      number: 9501,
      iteration_membership: { iteration_number: 1464, position: 100 }
    })
    const m2 = memberStory({
      number: 9502,
      iteration_membership: { iteration_number: 1464, position: 200 }
    })
    const html = renderToStaticMarkup(<Iteration stories={[reticulum, m1, m2]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-1464"')
    expect(html).toContain('Iteration Reticulum')
    expect(html).toContain('(2)')
  })

  test('Scenario 6 (regression-anchor): legacy-only member is dropped; only canonical-typed members count', () => {
    const caelum = iterationStory({ number: 1491, title: 'Iteration Caelum', gh_state: 'open' })
    const legacyOnly = memberStory({
      number: 9601,
      labels: ['Feature', iterationLabel('Caelum', 100)]
    })
    const canonicalOnly = memberStory({
      number: 9602,
      iteration_membership: { iteration_number: 1491, position: 200 }
    })
    const both = memberStory({
      number: 9603,
      labels: ['Feature', iterationLabel('Caelum', 300)],
      iteration_membership: { iteration_number: 1491, position: 300 }
    })
    const html = renderToStaticMarkup(
      <Iteration stories={[caelum, legacyOnly, canonicalOnly, both]} onOpen={() => {}} />
    )
    expect(html).toContain('data-test="board-iteration-1491"')
    expect(html).toContain('Iteration Caelum')
    expect(html).toContain('(2)')
  })

  test('Scenario 7 (drill-in-axis): canonical-typed members enumerated in member-list', () => {
    const microscopium = iterationStory({
      number: 1398,
      title: 'Iteration Microscopium',
      gh_state: 'open'
    })
    const m1 = memberStory({
      number: 9701,
      title: 'canonical-member-one',
      iteration_membership: { iteration_number: 1398, position: 100 }
    })
    const html = renderToStaticMarkup(<Iteration stories={[microscopium, m1]} onOpen={() => {}} />)
    expect(html).toContain('canonical-member-one')
  })

  test('Scenario 8 (other-iteration-isolation): membership for iter A does NOT count toward iter B', () => {
    const reticulum = iterationStory({
      number: 1464,
      title: 'Iteration Reticulum',
      gh_state: 'open'
    })
    const caelum = iterationStory({ number: 1491, title: 'Iteration Caelum', gh_state: 'open' })
    const memberOfCaelum = memberStory({
      number: 9801,
      iteration_membership: { iteration_number: 1491, position: 100 }
    })
    const html = renderToStaticMarkup(
      <Iteration stories={[reticulum, caelum, memberOfCaelum]} onOpen={() => {}} />
    )
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

describe('#1547 facet 1 — member workflow-state is legible in the iteration-view', () => {
  test('a ready member renders a visible state label hooked on data-state', () => {
    const crux = iterationStory({ number: 1543, title: 'Iteration Crux', gh_state: 'open' })
    const readyMember = memberStory({
      number: 1537,
      title: 'ready-member',
      state: 'ready',
      iteration_membership: { iteration_number: 1543, position: 100 }
    })
    const html = renderToStaticMarkup(<Iteration stories={[crux, readyMember]} onOpen={() => {}} />)
    expect(html).toContain('ready-member')
    expect(html).toContain('board-iteration-member-state')
    expect(html).toContain('data-state="ready"')
    expect(html).toContain('>Ready<')
  })

  test('in-progress and desk-check members carry their own legible state labels', () => {
    const crux = iterationStory({ number: 1543, title: 'Iteration Crux', gh_state: 'open' })
    const inProgress = memberStory({
      number: 1538,
      title: 'in-progress-member',
      state: 'in-progress',
      iteration_membership: { iteration_number: 1543, position: 100 }
    })
    const deskCheck = memberStory({
      number: 1539,
      title: 'desk-check-member',
      state: 'desk-check',
      iteration_membership: { iteration_number: 1543, position: 200 }
    })
    const html = renderToStaticMarkup(
      <Iteration stories={[crux, inProgress, deskCheck]} onOpen={() => {}} />
    )
    expect(html).toContain('data-state="in-progress"')
    expect(html).toContain('>In Progress<')
    expect(html).toContain('data-state="desk-check"')
    expect(html).toContain('>Desk Check<')
  })

  test('an icebox member stays neutral — no active-state label (icebox is the planned default)', () => {
    const crux = iterationStory({ number: 1543, title: 'Iteration Crux', gh_state: 'open' })
    const iceboxMember = memberStory({
      number: 1540,
      title: 'icebox-member',
      state: 'icebox',
      iteration_membership: { iteration_number: 1543, position: 100 }
    })
    const html = renderToStaticMarkup(
      <Iteration stories={[crux, iceboxMember]} onOpen={() => {}} />
    )
    expect(html).toContain('icebox-member')
    expect(html).not.toContain('board-iteration-member-state')
  })
})

describe('#1454 Iteration list scaling — section-grouped (Active+Planned / Closed)', () => {
  test('Scenario 1: renders 2 sub-sections with canonical labels + counts', () => {
    const active1 = iterationStory({
      number: 9001,
      title: 'Iteration ActiveA',
      gh_state: 'open'
    })
    const active2 = iterationStory({
      number: 9002,
      title: 'Iteration ActiveB',
      gh_state: 'open'
    })
    const closed1 = iterationStory({
      number: 9003,
      title: 'Iteration ClosedA',
      gh_state: 'closed'
    })
    const html = renderToStaticMarkup(
      <Iteration stories={[active1, active2, closed1]} onOpen={() => {}} />
    )
    expect(html).toContain('data-test="board-iterations-section-active-planned"')
    expect(html).toContain('data-test="board-iterations-section-closed"')
    expect(html).toContain('Active + Planned')
    expect(html).toContain('Closed')
    const activeCountIdx = html.indexOf('board-iterations-section-active-planned-count')
    const closedCountIdx = html.indexOf('board-iterations-section-closed-count')
    expect(html.slice(activeCountIdx, activeCountIdx + 200)).toContain('(2)')
    expect(html.slice(closedCountIdx, closedCountIdx + 200)).toContain('(1)')
  })

  test('Scenario 2: Active+Planned section open by default; Closed collapsed', () => {
    const active = iterationStory({ number: 9001, title: 'Iteration A', gh_state: 'open' })
    const closed = iterationStory({ number: 9002, title: 'Iteration B', gh_state: 'closed' })
    const html = renderToStaticMarkup(<Iteration stories={[active, closed]} onOpen={() => {}} />)
    const activeMatch = html.match(
      /data-test="board-iterations-section-active-planned"[^>]*open[^>]*>/
    )
    const closedMatch = html.match(/data-test="board-iterations-section-closed"([^>]*)>/)
    expect(activeMatch).toBeTruthy()
    expect(closedMatch?.[1]).not.toContain(' open')
  })

  test('Scenario 4: each section count reflects partition counts', () => {
    const a = iterationStory({ number: 9001, title: 'A', gh_state: 'open' })
    const b = iterationStory({ number: 9002, title: 'B', gh_state: 'open' })
    const c = iterationStory({ number: 9003, title: 'C', gh_state: 'open' })
    const x = iterationStory({ number: 9101, title: 'X', gh_state: 'closed' })
    const y = iterationStory({ number: 9102, title: 'Y', gh_state: 'closed' })
    const html = renderToStaticMarkup(<Iteration stories={[a, b, c, x, y]} onOpen={() => {}} />)
    const activeCountIdx = html.indexOf('board-iterations-section-active-planned-count')
    const closedCountIdx = html.indexOf('board-iterations-section-closed-count')
    expect(html.slice(activeCountIdx, activeCountIdx + 200)).toContain('(3)')
    expect(html.slice(closedCountIdx, closedCountIdx + 200)).toContain('(2)')
  })

  test('#1540: no iteration is anchored when none is designated (honest-empty, no auto-pick)', () => {
    const first = iterationStory({
      number: 9001,
      title: 'Iteration First',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.100']
    })
    const second = iterationStory({
      number: 9002,
      title: 'Iteration Second',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.200']
    })
    const html = renderToStaticMarkup(<Iteration stories={[second, first]} onOpen={() => {}} />)
    expect(html.match(/data-current-iteration="true"/)).toBeNull()
  })

  test('#1540: the explicitly-designated iteration is anchored regardless of order or in-flight children', () => {
    const iterFirst = iterationStory({
      number: 9001,
      title: 'Iteration First',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.100']
    })
    const iterSecond = iterationStory({
      number: 9002,
      title: 'Iteration Second',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.200'],
      is_current: true
    })
    const firstIceboxChild = memberStory({
      number: 9501,
      state: 'icebox',
      iteration_membership: { iteration_number: 9001, position: 100 }
    })
    const secondInFlightChild = memberStory({
      number: 9502,
      state: 'in-progress',
      iteration_membership: { iteration_number: 9002, position: 100 }
    })
    const html = renderToStaticMarkup(
      <Iteration
        stories={[iterFirst, iterSecond, firstIceboxChild, secondInFlightChild]}
        onOpen={() => {}}
      />
    )
    expect(
      html.match(/data-test="board-iteration-9002"[^>]*data-current-iteration="true"/)
    ).toBeTruthy()
    expect(
      html.match(/data-test="board-iteration-9001"[^>]*data-current-iteration="true"/)
    ).toBeNull()
  })

  test('Scenario regression: closed iterations omit drag-handle (grip)', () => {
    const closed = iterationStory({ number: 9001, title: 'Iteration ClosedA', gh_state: 'closed' })
    const html = renderToStaticMarkup(<Iteration stories={[closed]} onOpen={() => {}} />)
    const closedCardChunk = html.slice(html.indexOf('data-test="board-iteration-9001"'))
    expect(closedCardChunk).not.toContain('data-test="iteration-grip"')
  })
})

describe('#1517 iteration-row redesign — summary toggle-only + grip-sibling-right + navigate-in-body', () => {
  const summaryOf = (html: string): string => {
    const start = html.indexOf('<summary class="board-iteration-summary"')
    const end = html.indexOf('</summary>', start)
    return html.slice(start, end)
  }

  test('summary is toggle-only — no nested interactive element (grip + navigate moved out)', () => {
    const lyra = iterationStory({ number: 9000, title: 'Iteration Lyra', gh_state: 'open' })
    const html = renderToStaticMarkup(<Iteration stories={[lyra]} onOpen={() => {}} />)
    expect(summaryOf(html)).not.toContain('<button')
  })

  test('grip is a SIBLING of <details> (outside the summary), within the iteration-row parent', () => {
    const lyra = iterationStory({ number: 9000, title: 'Iteration Lyra', gh_state: 'open' })
    const html = renderToStaticMarkup(<Iteration stories={[lyra]} onOpen={() => {}} />)
    expect(html).toContain('class="board-iteration-row"')
    expect(summaryOf(html)).not.toContain('data-test="iteration-grip"')
    const gripIdx = html.indexOf('data-test="iteration-grip"')
    const detailsCloseIdx = html.indexOf('</details>')
    expect(gripIdx).toBeGreaterThan(detailsCloseIdx)
  })

  test('navigate (view iteration details) lives in the body, not the summary', () => {
    const lyra = iterationStory({ number: 9000, title: 'Iteration Lyra', gh_state: 'open' })
    const html = renderToStaticMarkup(<Iteration stories={[lyra]} onOpen={() => {}} />)
    expect(html).toContain('view iteration details')
    expect(summaryOf(html)).not.toContain('view iteration details')
  })

  test('grip is a keyboard-operable reorder control (tabindex=0 + aria-keyshortcuts)', () => {
    const lyra = iterationStory({ number: 9000, title: 'Iteration Lyra', gh_state: 'open' })
    const html = renderToStaticMarkup(<Iteration stories={[lyra]} onOpen={() => {}} />)
    const gripIdx = html.indexOf('data-test="iteration-grip"')
    const gripTagStart = html.lastIndexOf('<button', gripIdx)
    const gripTag = html.slice(gripTagStart, html.indexOf('>', gripIdx))
    expect(gripTag).toContain('tabindex="0"')
    expect(gripTag).toContain('aria-keyshortcuts="Control+Shift+ArrowUp Control+Shift+ArrowDown"')
  })
})

describe('#1517 iteration-row drag-sort adopts the shared useDragReorder (real per-row drop)', () => {
  afterEach(() => {
    cleanup()
    postLabelMutationMock.mockClear()
  })

  test('dragging an iteration grip and dropping on another iteration fires the order mutation', () => {
    const a = iterationStory({
      number: 9001,
      title: 'Iteration A',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(100)]
    })
    const b = iterationStory({
      number: 9002,
      title: 'Iteration B',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(200)]
    })
    render(<Iteration stories={[a, b]} onOpen={() => {}} />)

    const grips = screen.getAllByTestId('iteration-grip')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grips[0], { dataTransfer })

    const rows = document.querySelectorAll('.board-iteration-row')
    fireEvent.dragOver(rows[1] as HTMLElement, { dataTransfer })
    fireEvent.drop(rows[1] as HTMLElement, { dataTransfer })

    expect(postLabelMutationMock).toHaveBeenCalledTimes(1)
    const [movedNumber, addLabels, removeLabels] = postLabelMutationMock.mock.calls[0]
    expect(movedNumber).toBe(9001)
    expect(removeLabels).toEqual([iterationOrderLabel(100)])
    const newPos = Number(String(addLabels[0]).replace('iteration-order.', ''))
    expect(newPos).toBeGreaterThan(200)
  })

  test('clause-13: a dropped iteration stays in the new order in the optimistic window, before the backend confirms (no snap-back to the label order)', () => {
    const a = iterationStory({
      number: 9001,
      title: 'Iteration A',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(100)]
    })
    const b = iterationStory({
      number: 9002,
      title: 'Iteration B',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(200)]
    })
    render(<Iteration stories={[a, b]} onOpen={() => {}} />)
    expect(activeIterationNumbers()).toEqual([9001, 9002])

    const grips = screen.getAllByTestId('iteration-grip')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grips[0], { dataTransfer })
    const rows = document.querySelectorAll('.board-iteration-row')
    fireEvent.dragOver(rows[1] as HTMLElement, { dataTransfer })
    fireEvent.drop(rows[1] as HTMLElement, { dataTransfer })

    expect(activeIterationNumbers()).toEqual([9002, 9001])
  })

  test('#1517 dragging an UNORDERED iteration (no iteration-order label — how create_iteration makes them) establishes a persisted order instead of silently no-opping', () => {
    const a = iterationStory({
      number: 9001,
      title: 'Iteration A',
      gh_state: 'open',
      labels: ['Iteration']
    })
    const b = iterationStory({
      number: 9002,
      title: 'Iteration B',
      gh_state: 'open',
      labels: ['Iteration']
    })
    render(<Iteration stories={[a, b]} onOpen={() => {}} />)
    expect(activeIterationNumbers()).toEqual([9001, 9002])

    const grips = screen.getAllByTestId('iteration-grip')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grips[1], { dataTransfer })
    const rows = document.querySelectorAll('.board-iteration-row')
    fireEvent.dragOver(rows[0] as HTMLElement, { dataTransfer })
    fireEvent.drop(rows[0] as HTMLElement, { dataTransfer })

    const calledNumbers = postLabelMutationMock.mock.calls.map((c) => c[0])
    expect(calledNumbers).toContain(9001)
    expect(calledNumbers).toContain(9002)
    expect(activeIterationNumbers()).toEqual([9002, 9001])
  })

  test('#1517 a reorder clears ALL stale order-labels on the dragged iteration (self-heals accumulated duplicates like Aquila #1655)', () => {
    const accumulated = iterationStory({
      number: 9001,
      title: 'Iteration A',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(100), iterationOrderLabel(250)]
    })
    const other = iterationStory({
      number: 9002,
      title: 'Iteration B',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(200)]
    })
    render(<Iteration stories={[accumulated, other]} onOpen={() => {}} />)
    expect(activeIterationNumbers()).toEqual([9001, 9002])

    const grips = screen.getAllByTestId('iteration-grip')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grips[0], { dataTransfer })
    const rows = document.querySelectorAll('.board-iteration-row')
    fireEvent.dragOver(rows[1] as HTMLElement, { dataTransfer })
    fireEvent.drop(rows[1] as HTMLElement, { dataTransfer })

    const call = postLabelMutationMock.mock.calls.find((c) => c[0] === 9001)
    expect(call).toBeDefined()
    const removeLabels = call?.[2] as string[]
    expect(removeLabels).toContain(iterationOrderLabel(100))
    expect(removeLabels).toContain(iterationOrderLabel(250))
  })

  test('closed iterations are not drag sources (no grip) — drag-sort is active-planned only', () => {
    const open = iterationStory({
      number: 9003,
      title: 'Open Iteration',
      gh_state: 'open',
      labels: ['Iteration', iterationOrderLabel(100)]
    })
    const closed = iterationStory({
      number: 9004,
      title: 'Closed Iteration',
      gh_state: 'closed',
      labels: ['Iteration', iterationOrderLabel(200)]
    })
    render(<Iteration stories={[open, closed]} onOpen={() => {}} />)
    expect(screen.getAllByTestId('iteration-grip')).toHaveLength(1)
  })
})
