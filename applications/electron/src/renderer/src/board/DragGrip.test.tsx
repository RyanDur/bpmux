import { afterEach, describe, expect, test, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/react'
import { DragGrip } from './DragGrip'
import { STORY_GRIP_TEST_ID } from './testIds'

afterEach(cleanup)

describe('DragGrip', () => {
  test('renders a non-tabbable grip button with reorder aria-label + grip test id', () => {
    const { getByTestId } = render(
      <DragGrip
        storyNumber={42}
        gripDragProps={{ draggable: true, onDragStart: () => {}, onDragEnd: () => {} }}
      />
    )
    const grip = getByTestId(STORY_GRIP_TEST_ID)
    expect(grip.tagName).toBe('BUTTON')
    expect(grip.getAttribute('type')).toBe('button')
    expect(grip.getAttribute('aria-label')).toBe('Drag to reorder #42')
    expect(grip.getAttribute('tabindex')).toBe('-1')
    expect(grip.getAttribute('draggable')).toBe('true')
  })

  test('wires drag handlers from gripDragProps', () => {
    const onDragStart = vi.fn()
    const onDragEnd = vi.fn()
    const { getByTestId } = render(
      <DragGrip storyNumber={7} gripDragProps={{ draggable: true, onDragStart, onDragEnd }} />
    )
    const grip = getByTestId(STORY_GRIP_TEST_ID)
    fireEvent.dragStart(grip)
    expect(onDragStart).toHaveBeenCalledTimes(1)
    fireEvent.dragEnd(grip)
    expect(onDragEnd).toHaveBeenCalledTimes(1)
  })
})
