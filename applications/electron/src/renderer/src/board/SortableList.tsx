import { Fragment } from 'react'
import { useDragReorder, type GripDragProps, type RowDragProps } from './useDragReorder'
import type { DragMimeType } from './useStoryDrag'

export interface SortableDragConfig<P> {
  mimeType: DragMimeType
  encode: (payload: P) => string
  decode: (raw: string) => P | null
  payloadKey: (payload: P) => number
}

export interface SortableRowContext {
  rowDragProps: RowDragProps
  gripDragProps: GripDragProps
  dragging: boolean
  index: number
}

export interface SortableListProps<P, T> {
  items: T[]
  keyOf: (item: T) => number
  rowKey: (item: T) => React.Key
  config: SortableDragConfig<P>
  buildPayload: (item: T) => P
  onReorder: (payload: P, targetKey: number) => void
  disabled?: boolean
  ulClassName: string
  renderRow: (item: T, context: SortableRowContext) => React.ReactNode
}

export const SortableList = <P, T>({
  items,
  keyOf,
  rowKey,
  config,
  buildPayload,
  onReorder,
  disabled,
  ulClassName,
  renderRow
}: SortableListProps<P, T>): React.JSX.Element => {
  const { orderedItems, draggedNumber, rowProps, gripProps } = useDragReorder<P, T>({
    items,
    keyOf,
    mimeType: config.mimeType,
    encode: config.encode,
    decode: config.decode,
    payloadKey: config.payloadKey,
    buildPayload,
    onReorder,
    disabled
  })
  return (
    <ul className={ulClassName}>
      {orderedItems.map((item, index) => (
        <Fragment key={rowKey(item)}>
          {renderRow(item, {
            rowDragProps: rowProps(item),
            gripDragProps: gripProps(item),
            dragging: draggedNumber === keyOf(item),
            index
          })}
        </Fragment>
      ))}
    </ul>
  )
}
