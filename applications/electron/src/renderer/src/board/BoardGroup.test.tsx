import { afterEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import BoardGroup from './BoardGroup'
import type { BoardStory, GroupSpec } from './types'

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
    const html = renderToStaticMarkup(<BoardGroup {...baseProps} stories={[]} loading={true} />)
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
      <BoardGroup {...baseProps} group={acceptedGroup} stories={stories} collapsed={true} />
    )
    expect(html).toContain('data-test="board-group-accepted"')
    expect(html).toContain('shipped')
  })
})

describe('#1524 ready-queue real per-row drop fires onReorder(state, moved, targetNumber)', () => {
  afterEach(cleanup)

  test('dragging a grip and dropping onto a sibling row reorders (not the dropAt/ul path)', () => {
    const onReorder = vi.fn()
    const stories = [
      story({ number: 701, title: 'first' }),
      story({ number: 702, title: 'second' }),
      story({ number: 703, title: 'third' })
    ]
    render(<BoardGroup {...baseProps} stories={stories} onReorder={onReorder} />)

    const grip = screen.getByLabelText('Drag to reorder #701')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grip, { dataTransfer })

    const targetRow = document.querySelector('[data-test="board-story-702"]') as HTMLElement
    fireEvent.dragOver(targetRow, { dataTransfer })
    fireEvent.drop(targetRow, { dataTransfer })

    expect(onReorder).toHaveBeenCalledWith('ready', 701, 702)
  })

  test('dropping onto a non-adjacent row targets that row', () => {
    const onReorder = vi.fn()
    const stories = [
      story({ number: 701, title: 'first' }),
      story({ number: 702, title: 'second' }),
      story({ number: 703, title: 'third' })
    ]
    render(<BoardGroup {...baseProps} stories={stories} onReorder={onReorder} />)

    const grip = screen.getByLabelText('Drag to reorder #701')
    const dataTransfer = makeDataTransfer()
    fireEvent.dragStart(grip, { dataTransfer })

    const targetRow = document.querySelector('[data-test="board-story-703"]') as HTMLElement
    fireEvent.dragOver(targetRow, { dataTransfer })
    fireEvent.drop(targetRow, { dataTransfer })

    expect(onReorder).toHaveBeenCalledWith('ready', 701, 703)
  })
})
