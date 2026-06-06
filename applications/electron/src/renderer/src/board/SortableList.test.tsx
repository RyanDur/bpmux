import { afterEach, describe, expect, test, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { SortableList, type SortableDragConfig } from './SortableList'
import { DRAG_MIMETYPE_REGISTRY } from './useStoryDrag'

afterEach(cleanup)

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

const numberConfig: SortableDragConfig<number> = {
  mimeType: DRAG_MIMETYPE_REGISTRY.STORY_ROW,
  encode: String,
  decode: (raw) => {
    const n = Number.parseInt(raw, 10)
    return Number.isNaN(n) ? null : n
  },
  payloadKey: (n) => n
}

const renderList = (onReorder = vi.fn()): void => {
  render(
    <SortableList<number, number>
      items={[1, 2, 3]}
      keyOf={(n) => n}
      rowKey={(n) => n}
      config={numberConfig}
      buildPayload={(n) => n}
      onReorder={onReorder}
      ulClassName="test-list"
      renderRow={(n, ctx) => (
        <li
          data-test={`row-${n}`}
          data-dragging={ctx.dragging ? 'true' : undefined}
          {...ctx.rowDragProps}
        >
          <button type="button" data-test={`grip-${n}`} {...ctx.gripDragProps} />
        </li>
      )}
    />
  )
}

describe('SortableList', () => {
  test('renders each item through renderRow inside a ul carrying the shared sortable-list class plus the given class', () => {
    renderList()
    const list = screen.getByRole('list')
    expect(list.classList.contains('sortable-list')).toBe(true)
    expect(list.classList.contains('test-list')).toBe(true)
    expect(screen.queryByTestId('row-1')).not.toBeNull()
    expect(screen.queryByTestId('row-2')).not.toBeNull()
    expect(screen.queryByTestId('row-3')).not.toBeNull()
  })

  test('fires onReorder with the decoded payload + target key on a real drop', () => {
    const onReorder = vi.fn()
    renderList(onReorder)
    const dt = makeDataTransfer()
    fireEvent.dragStart(screen.getByTestId('grip-1'), { dataTransfer: dt })
    fireEvent.dragOver(screen.getByTestId('row-3'), { dataTransfer: dt })
    fireEvent.drop(screen.getByTestId('row-3'), { dataTransfer: dt })
    expect(onReorder).toHaveBeenCalledWith(1, 3)
  })

  test('marks the dragged row via the dragging context flag', () => {
    renderList()
    const dt = makeDataTransfer()
    fireEvent.dragStart(screen.getByTestId('grip-2'), { dataTransfer: dt })
    expect(screen.getByTestId('row-2').getAttribute('data-dragging')).toBe('true')
  })

  test('passes each row its ordered index through the context', () => {
    render(
      <SortableList<number, number>
        items={[10, 20, 30]}
        keyOf={(n) => n}
        rowKey={(n) => n}
        config={numberConfig}
        buildPayload={(n) => n}
        onReorder={vi.fn()}
        ulClassName="indexed-list"
        renderRow={(n, ctx) => <li data-test={`indexed-${n}`} data-index={ctx.index} />}
      />
    )
    expect(screen.getByTestId('indexed-10').getAttribute('data-index')).toBe('0')
    expect(screen.getByTestId('indexed-20').getAttribute('data-index')).toBe('1')
    expect(screen.getByTestId('indexed-30').getAttribute('data-index')).toBe('2')
  })
})
