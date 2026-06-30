import { describe, expect, test } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { DRAG_MIMETYPE_REGISTRY, useStoryDrag } from './useStoryDrag'

const stringOptions = {
  mimeType: DRAG_MIMETYPE_REGISTRY.STORY_ROW,
  encode: String,
  decode: (s: string): number | null => {
    const n = Number.parseInt(s, 10)
    return Number.isNaN(n) ? null : n
  }
}

const fakeDragEvent = (overrides: Partial<DataTransfer> = {}): React.DragEvent<HTMLElement> => {
  const stored: Record<string, string> = {}
  const types: string[] = []
  const dataTransfer = {
    setData: (key: string, value: string) => {
      stored[key] = value
      if (!types.includes(key)) types.push(key)
    },
    getData: (key: string): string => stored[key] ?? '',
    setDragImage: () => {},
    effectAllowed: 'none' as DataTransfer['effectAllowed'],
    dropEffect: 'none' as DataTransfer['dropEffect'],
    types: types as unknown as DataTransferItemList & string[],
    ...overrides
  }
  return {
    dataTransfer: dataTransfer as unknown as DataTransfer,
    currentTarget: { closest: () => null } as unknown as HTMLElement,
    clientX: 0,
    clientY: 0
  } as unknown as React.DragEvent<HTMLElement>
}

describe('#1358 useStoryDrag lifecycle', () => {
  test('initial state: draggedId and hoveredId are null', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    expect(result.current.draggedId).toBeNull()
    expect(result.current.hoveredId).toBeNull()
  })

  test('startDrag sets draggedId and writes payload to DataTransfer with mimeType', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    const event = fakeDragEvent()
    act(() => result.current.startDrag(event, 42, 42))
    expect(result.current.draggedId).toBe(42)
    expect(event.dataTransfer.getData(DRAG_MIMETYPE_REGISTRY.STORY_ROW)).toBe('42')
    expect(event.dataTransfer.effectAllowed).toBe('move')
  })

  test('endDrag clears both draggedId and hoveredId', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    act(() => result.current.startDrag(fakeDragEvent(), 7, 7))
    act(() => result.current.hover(9))
    expect(result.current.draggedId).toBe(7)
    expect(result.current.hoveredId).toBe(9)
    act(() => result.current.endDrag())
    expect(result.current.draggedId).toBeNull()
    expect(result.current.hoveredId).toBeNull()
  })

  test('hover updates hoveredId without touching draggedId', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    act(() => result.current.startDrag(fakeDragEvent(), 100, 100))
    act(() => result.current.hover(200))
    expect(result.current.draggedId).toBe(100)
    expect(result.current.hoveredId).toBe(200)
    act(() => result.current.hover(null))
    expect(result.current.hoveredId).toBeNull()
    expect(result.current.draggedId).toBe(100)
  })

  test('decode reads payload back through caller-supplied decoder', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    const event = fakeDragEvent()
    act(() => result.current.startDrag(event, 55, 55))
    expect(result.current.decode(event)).toBe(55)
  })

  test('decode returns null when DataTransfer has no payload for the mimeType', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    expect(result.current.decode(fakeDragEvent())).toBeNull()
  })

  test('hasType reflects whether the configured mimeType is set on the event', () => {
    const { result } = renderHook(() => useStoryDrag(stringOptions))
    const event = fakeDragEvent()
    expect(result.current.hasType(event)).toBe(false)
    act(() => result.current.startDrag(event, 1, 1))
    expect(result.current.hasType(event)).toBe(true)
  })

  test('encoder + decoder round-trip a JSON payload type', () => {
    interface Payload {
      storyId: number
      name: string
    }
    const { result } = renderHook(() =>
      useStoryDrag<Payload>({
        mimeType: DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER,
        encode: JSON.stringify,
        decode: (s) => {
          try {
            return JSON.parse(s) as Payload
          } catch {
            return null
          }
        }
      })
    )
    const event = fakeDragEvent()
    const payload: Payload = { storyId: 9, name: 'Andromeda' }
    act(() => result.current.startDrag(event, payload.storyId, payload))
    expect(result.current.decode(event)).toEqual(payload)
  })

  test('#1517 startDrag sets the drag image from a detached, test-id-stripped CLONE of the row — not the live row that gets opacity-hidden (so the ghost stays visible)', () => {
    const li = document.createElement('li')
    li.setAttribute('data-test', 'board-iteration-42')
    document.body.appendChild(li)
    const stored: Record<string, string> = {}
    let dragImageArg: HTMLElement | null = null
    const event = {
      dataTransfer: {
        setData: (key: string, value: string) => {
          stored[key] = value
        },
        getData: (key: string): string => stored[key] ?? '',
        setDragImage: (image: HTMLElement) => {
          dragImageArg = image
        },
        effectAllowed: 'none',
        dropEffect: 'none',
        types: [] as unknown as DataTransferItemList & string[]
      },
      currentTarget: li,
      clientX: 0,
      clientY: 0
    } as unknown as React.DragEvent<HTMLElement>

    const { result } = renderHook(() => useStoryDrag(stringOptions))
    act(() => result.current.startDrag(event, 42, 42))

    expect(dragImageArg).not.toBeNull()
    expect(dragImageArg).not.toBe(li)
    expect((dragImageArg as HTMLElement | null)?.getAttribute('data-test')).toBeNull()
    li.remove()
  })

  test('DRAG_MIMETYPE_REGISTRY exports the 3 canonical mime types', () => {
    expect(DRAG_MIMETYPE_REGISTRY.STORY_ROW).toBe('application/x-bpmux-story')
    expect(DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER).toBe('application/x-bpmux-iteration-member')
    expect(DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF).toBe('application/x-bpmux-iteration-itself')
  })
})
