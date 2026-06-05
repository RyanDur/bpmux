import { eagerReorder } from './iterationMembers'
import { useStoryDrag, type UseStoryDragOptions } from './useStoryDrag'

export interface UseDragReorderOptions<P, T> extends UseStoryDragOptions<P> {
  items: T[]
  keyOf: (item: T) => number
  payloadKey: (payload: P) => number
  buildPayload: (item: T) => P
  onReorder: (payload: P, targetKey: number) => void
  disabled?: boolean
}

export interface RowDragProps {
  onDragOver: (event: React.DragEvent<HTMLElement>) => void
  onDrop: (event: React.DragEvent<HTMLElement>) => void
}

export interface GripDragProps {
  draggable: true
  onDragStart: (event: React.DragEvent<HTMLElement>) => void
  onDragEnd: () => void
}

export interface UseDragReorderResult<T> {
  orderedItems: T[]
  draggedNumber: number | null
  hoveredNumber: number | null
  rowProps: (item: T) => RowDragProps
  gripProps: (item: T) => GripDragProps
}

export const useDragReorder = <P, T>(
  options: UseDragReorderOptions<P, T>
): UseDragReorderResult<T> => {
  const { items, keyOf, payloadKey, buildPayload, onReorder, disabled = false } = options
  const { draggedNumber, hoveredNumber, startDrag, endDrag, hover, decode, hasType } =
    useStoryDrag<P>(options)

  const orderedItems = eagerReorder(items, keyOf, draggedNumber, hoveredNumber)

  const rowProps = (item: T): RowDragProps => ({
    onDragOver: (event) => {
      if (!hasType(event)) return
      event.preventDefault()
      event.dataTransfer.dropEffect = 'move'
      const key = keyOf(item)
      if (draggedNumber !== null && draggedNumber !== key) hover(key)
    },
    onDrop: (event) => {
      const payload = decode(event)
      if (payload === null) return
      event.preventDefault()
      if (disabled) return
      const key = keyOf(item)
      const targetKey = payloadKey(payload) === key && hoveredNumber !== null ? hoveredNumber : key
      if (payloadKey(payload) === targetKey) return
      onReorder(payload, targetKey)
      endDrag()
    }
  })

  const gripProps = (item: T): GripDragProps => ({
    draggable: true,
    onDragStart: (event) => startDrag(event, keyOf(item), buildPayload(item)),
    onDragEnd: endDrag
  })

  return { orderedItems, draggedNumber, hoveredNumber, rowProps, gripProps }
}
