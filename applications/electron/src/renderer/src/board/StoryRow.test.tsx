import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import StoryRow from './StoryRow'
import { type BoardStory } from './types'

const featureLabels = ['Feature']

const story = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 1,
  title: 'stub',
  state: 'ready',
  labels: featureLabels,
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: '',
  ...overrides
})

const baseProps = {
  selected: false,
  draggable: false,
  dragging: false,
  position: 1,
  total: 1,
  onOpen: () => {},
  onSelect: () => {}
}

describe('#1260 story-row hierarchy + color', () => {
  test('story-title is the primary visible text and carries data-test="story-title"', () => {
    const s = story({ number: 1258, title: 'close_iteration marker-promote extension' })
    const html = renderToStaticMarkup(<StoryRow story={s} {...baseProps} />)
    expect(html).toContain('data-test="story-title"')
    expect(html).toContain('close_iteration marker-promote extension')
    expect(html).toMatch(/<b[^>]*data-test="story-title"[^>]*>close_iteration/)
  })

  test('epic-link renders as `↳ Epic #N` only — no full Epic title in visible text (falsifier-a, b)', () => {
    const s = story({ number: 1258, parent_epic: 1189 })
    const html = renderToStaticMarkup(
      <StoryRow
        story={s}
        parentEpicTitle="Iteration markers: group icebox stories"
        {...baseProps}
      />
    )
    expect(html).toContain('data-test="epic-link"')
    const epicLinkVisible = html.match(/<button[^>]*data-test="epic-link"[^>]*>([^<]*)<\/button>/)
    expect(epicLinkVisible?.[1]).toBe('↳ Epic #1189')
    expect(epicLinkVisible?.[1]).not.toContain('Iteration markers: group icebox stories')
  })

  test('epic-link is a button (clickable affordance) (AC3)', () => {
    const s = story({ number: 1258, parent_epic: 1189 })
    const html = renderToStaticMarkup(<StoryRow story={s} {...baseProps} />)
    expect(html).toMatch(/<button[^>]*data-test="epic-link"/)
    expect(html).toMatch(/<button[^>]*type="button"/)
  })

  test('epic-link omitted when parent_epic is undefined', () => {
    const s = story({ number: 1258 })
    const html = renderToStaticMarkup(<StoryRow story={s} {...baseProps} />)
    expect(html).not.toContain('data-test="epic-link"')
    expect(html).not.toContain('board-story-epic-ref')
  })

  test('epic-link omitted when parent_epic is null (server-side null leak)', () => {
    const s = { ...story({ number: 1258 }), parent_epic: null as unknown as number }
    const html = renderToStaticMarkup(<StoryRow story={s} {...baseProps} />)
    expect(html).not.toContain('data-test="epic-link"')
    expect(html).not.toContain('↳ Epic')
  })

  test('epic-link aria-label still names parent epic + title for screen readers', () => {
    const s = story({ number: 1258, parent_epic: 1189 })
    const html = renderToStaticMarkup(
      <StoryRow
        story={s}
        parentEpicTitle="Iteration markers: group icebox stories"
        {...baseProps}
      />
    )
    expect(html).toMatch(/aria-label="open epic #1189 Iteration markers: group icebox stories"/)
  })

  test('epic-link aria-label falls back when parentEpicTitle absent', () => {
    const s = story({ number: 1258, parent_epic: 1189 })
    const html = renderToStaticMarkup(<StoryRow story={s} {...baseProps} />)
    expect(html).toMatch(/aria-label="open epic #1189"/)
  })

  test('epic-link is rendered as a button (not a small/span) so click affordance is canonical', () => {
    const s = story({ number: 1258, parent_epic: 1189 })
    const html = renderToStaticMarkup(<StoryRow story={s} {...baseProps} />)
    const epicLinkMatch = html.match(/<button[^>]*data-test="epic-link"[^>]*>[^<]*<\/button>/)
    expect(epicLinkMatch).not.toBeNull()
    expect(epicLinkMatch?.[0]).toContain('↳ Epic #1189')
  })

  test('hierarchy applies across all column states (sister-surfaces; falsifier-i)', () => {
    const states: BoardStory['state'][] = [
      'ready',
      'in-progress',
      'desk-check',
      'delivered',
      'accepted'
    ]
    for (const state of states) {
      const s = story({ number: 1258, state, parent_epic: 1189 })
      const html = renderToStaticMarkup(
        <StoryRow story={s} parentEpicTitle="Some Epic Title" {...baseProps} />
      )
      expect(html).toContain('data-test="story-title"')
      expect(html).toContain('data-test="epic-link"')
      const epicLinkVisible = html.match(/<button[^>]*data-test="epic-link"[^>]*>([^<]*)<\/button>/)
      expect(epicLinkVisible?.[1]).not.toContain('Some Epic Title')
    }
  })
})
