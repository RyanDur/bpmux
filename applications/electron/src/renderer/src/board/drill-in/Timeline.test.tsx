import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Timeline from './Timeline'
import type { Story } from '../types'
const featureLabels = ['Feature']

const story = (overrides: Partial<Story>): Story => ({
  id: 700,
  title: 'stub story',
  state: 'ready',
  labels: featureLabels,
  assignees: [],
  created_at: '2026-05-09T00:00:00Z',
  body: '',
  ...overrides
})

const baseProps = {
  apiBase: undefined
}

describe('Timeline independence', () => {
  test('renders standalone (substrate-axis) — no DrillIn.tsx parent required', () => {
    const html = renderToStaticMarkup(<Timeline story={story({})} {...baseProps} />)
    expect(html).toContain('data-test="drill-in-timeline"')
    expect(html).toMatch(/<details[^>]*class="drill-in-timeline"/)
  })

  test('renders Timeline summary heading', () => {
    const html = renderToStaticMarkup(<Timeline story={story({})} {...baseProps} />)
    expect(html).toContain('<h4>Timeline')
  })

  test('renders "no events yet" empty state when apiBase=undefined', () => {
    const html = renderToStaticMarkup(<Timeline story={story({})} {...baseProps} />)
    expect(html).toContain('no events yet')
    expect(html).toContain('drill-in-timeline-empty')
  })

  test('does NOT render the show-all toggle when there are no events', () => {
    const html = renderToStaticMarkup(<Timeline story={story({})} {...baseProps} />)
    expect(html).not.toContain('drill-in-timeline-toggle')
  })

  test('details element is not auto-opened when focusTimeline is omitted', () => {
    const html = renderToStaticMarkup(<Timeline story={story({})} {...baseProps} />)
    expect(html).toMatch(/<details[^>]*class="drill-in-timeline"(?![^>]*open)/)
  })
})
