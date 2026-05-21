import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Iteration from './Iteration'
import { iterationLabel } from './labelSchema'
import type { BoardStory } from './types'

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
    const m1 = memberStory({ number: 9201, labels: ['Feature', iterationLabel('Carina', 100)] })
    const m2 = memberStory({ number: 9202, labels: ['Feature', iterationLabel('Carina', 200)] })
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
    const m1 = memberStory({ number: 9301, labels: ['Feature', iterationLabel('Andromeda', 100)] })
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

  test('Scenario 6 (regression-anchor): mixed legacy-label + canonical-typed members union + dedupe', () => {
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
    expect(html).toContain('(3)')
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

  test('Scenario 6 (fallback): first active iteration by iteration-order gets anchor when no iter has in-flight work', () => {
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
    const firstAnchor = html.match(
      /data-test="board-iteration-9001"[^>]*data-current-iteration="true"/
    )
    const secondAnchor = html.match(
      /data-test="board-iteration-9002"[^>]*data-current-iteration="true"/
    )
    expect(firstAnchor).toBeTruthy()
    expect(secondAnchor).toBeNull()
  })

  test('Scenario 6-rework (#1454): iter with only-iceboxed children NOT anchored; iter with in-flight children IS anchored', () => {
    const iterIcebox = iterationStory({
      number: 9001,
      title: 'Iteration IceboxOnly',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.100']
    })
    const iterInFlight = iterationStory({
      number: 9002,
      title: 'Iteration Working',
      gh_state: 'open',
      labels: ['iteration', 'iteration-order.200']
    })
    const iceboxChild = memberStory({
      number: 9501,
      state: 'icebox',
      iteration_membership: { iteration_number: 9001, position: 100 }
    })
    const inFlightChild = memberStory({
      number: 9502,
      state: 'in-progress',
      iteration_membership: { iteration_number: 9002, position: 100 }
    })
    const html = renderToStaticMarkup(
      <Iteration
        stories={[iterIcebox, iterInFlight, iceboxChild, inFlightChild]}
        onOpen={() => {}}
      />
    )
    expect(
      html.match(/data-test="board-iteration-9001"[^>]*data-current-iteration="true"/)
    ).toBeNull()
    expect(
      html.match(/data-test="board-iteration-9002"[^>]*data-current-iteration="true"/)
    ).toBeTruthy()
  })

  test('Scenario regression: closed iterations omit drag-handle (grip)', () => {
    const closed = iterationStory({ number: 9001, title: 'Iteration ClosedA', gh_state: 'closed' })
    const html = renderToStaticMarkup(<Iteration stories={[closed]} onOpen={() => {}} />)
    const closedCardChunk = html.slice(html.indexOf('data-test="board-iteration-9001"'))
    expect(closedCardChunk).not.toContain('data-test="iteration-grip"')
  })
})
