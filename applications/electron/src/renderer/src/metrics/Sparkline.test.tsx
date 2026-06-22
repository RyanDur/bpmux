import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import Sparkline, { type WipPoint } from './Sparkline'
import { sparkline } from './Sparkline.page'

afterEach(cleanup)

const series = (...counts: number[]): WipPoint[] =>
  counts.map((wip_count, i) => ({ date: `2026-06-${10 + i}`, wip_count }))

describe('#1237 WIP-count sparkline', () => {
  test('renders an accessible img with a trend-glance aria-label + a polyline point per data-point', () => {
    render(<Sparkline history={series(1, 2, 3, 2)} />)
    expect(sparkline.svg()).toBeTruthy()
    expect(sparkline.trendLabel()).toMatch(/WIP trend: 2 items, climbing over 4 days/)
    expect(sparkline.polylinePointCount()).toBe(4)
  })

  test('aria-label reads "falling" when the trend descends', () => {
    render(<Sparkline history={series(5, 3, 1)} />)
    expect(sparkline.trendLabel()).toMatch(/falling/)
  })

  test('a flat (>=2 equal points) series IS a steady line, not the empty-state', () => {
    render(<Sparkline history={series(3, 3, 3)} />)
    expect(sparkline.trendLabel()).toMatch(/steady/)
    expect(sparkline.svg()).toBeTruthy()
  })

  test('exposes an sr-only data-table of the points (non-visual detail path)', () => {
    render(<Sparkline history={series(1, 2, 3)} />)
    const table = sparkline.dataTable()
    expect(table).toBeTruthy()
    expect(table?.querySelectorAll('tbody tr').length).toBe(3)
  })

  test('Scenario 4: a single point renders the "no data yet" placeholder, never a flat-line', () => {
    render(<Sparkline history={series(4)} />)
    expect(sparkline.emptyPlaceholder()).toBeTruthy()
    expect(sparkline.svg()).toBeFalsy()
  })

  test('empty history renders the placeholder', () => {
    render(<Sparkline history={[]} />)
    expect(sparkline.emptyPlaceholder()).toBeTruthy()
    expect(sparkline.svg()).toBeFalsy()
  })
})
