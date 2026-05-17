import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import BoardGroup from './BoardGroup'
import type { BoardStory, GroupSpec } from './types'

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

const readyGroup: GroupSpec = {
  state: 'ready',
  label: 'Ready',
  collapsed: false
}

const baseProps = {
  group: readyGroup,
  epicTitleByNumber: new Map<number, string>(),
  loading: false,
  selectedNumber: null,
  onOpen: () => {},
  onSelect: () => {},
  collapsed: false,
  onToggleCollapse: () => {}
}

describe('#1357 BoardGroup independence', () => {
  test('renders standalone (substrate-axis) — no Board.tsx parent required', () => {
    const stories = [story({ number: 101, title: 'first ready story' })]
    const html = renderToStaticMarkup(<BoardGroup {...baseProps} stories={stories} />)
    expect(html).toContain('data-test="board-group-ready"')
    expect(html).toContain('first ready story')
  })

  test('renders empty-state "none" when stories is empty', () => {
    const html = renderToStaticMarkup(<BoardGroup {...baseProps} stories={[]} />)
    expect(html).toContain('board-group-empty')
    expect(html).toContain('none')
  })

  test('renders skeleton list when loading=true', () => {
    const html = renderToStaticMarkup(
      <BoardGroup {...baseProps} stories={[]} loading={true} />
    )
    expect(html).toContain('board-story-list--skeleton')
    expect(html).toContain('skeleton-bar')
  })

  test('renders count badge derived from stories.length', () => {
    const stories = [story({ number: 101 }), story({ number: 102 }), story({ number: 103 })]
    const html = renderToStaticMarkup(<BoardGroup {...baseProps} stories={stories} />)
    expect(html).toContain('data-test="board-group-ready-count"')
    expect(html).toContain('(3)')
  })

  test('respects collapsed=true (omits the open attribute on details)', () => {
    const stories = [story({ number: 101 })]
    const html = renderToStaticMarkup(
      <BoardGroup {...baseProps} stories={stories} collapsed={true} />
    )
    expect(html).toMatch(/<details[^>]*class="board-group"(?![^>]*open)/)
  })

  test('respects collapsed=false (open attribute present on details)', () => {
    const stories = [story({ number: 101 })]
    const html = renderToStaticMarkup(
      <BoardGroup {...baseProps} stories={stories} collapsed={false} />
    )
    expect(html).toMatch(/<details[^>]*class="board-group"[^>]*open/)
  })

  test('renders children prop in place of stories list', () => {
    const html = renderToStaticMarkup(
      <BoardGroup {...baseProps} stories={[]}>
        <section data-test="custom-child-content">child slot content</section>
      </BoardGroup>
    )
    expect(html).toContain('data-test="custom-child-content"')
    expect(html).toContain('child slot content')
    expect(html).not.toContain('board-group-empty')
  })

  test('non-draggable groups (e.g. accepted) still render stories', () => {
    const acceptedGroup: GroupSpec = {
      state: 'accepted',
      label: 'Accepted',
      collapsed: true
    }
    const stories = [story({ number: 101, state: 'accepted', title: 'shipped' })]
    const html = renderToStaticMarkup(
      <BoardGroup
        {...baseProps}
        group={acceptedGroup}
        stories={stories}
        collapsed={true}
      />
    )
    expect(html).toContain('data-test="board-group-accepted"')
    expect(html).toContain('shipped')
  })
})
