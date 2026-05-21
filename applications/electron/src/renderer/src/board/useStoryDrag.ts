import { useCallback, useState } from 'react'

export const DRAG_MIMETYPE_REGISTRY = {
  STORY_ROW: 'text/bpmux-story',
  ITERATION_MEMBER: 'application/x-bpmux-iteration-member',
  ITERATION_ITSELF: 'application/x-bpmux-iteration-itself'
} as const

export type DragMimeType = (typeof DRAG_MIMETYPE_REGISTRY)[keyof typeof DRAG_MIMETYPE_REGISTRY]

export interface UseStoryDragOptions<P> {
  mimeType: DragMimeType
  encode: (payload: P) => string
  decode: (raw: string) => P | null
}

export interface UseStoryDragResult<P> {
  draggedNumber: number | null
  hoveredNumber: number | null
  startDrag: (event: React.DragEvent<HTMLElement>, storyNumber: number, payload: P) => void
  endDrag: () => void
  hover: (storyNumber: number | null) => void
  decode: (event: React.DragEvent<HTMLElement>) => P | null
  hasType: (event: React.DragEvent<HTMLElement>) => boolean
}

const setDragImageFromClosestLi = (event: React.DragEvent<HTMLElement>): void => {
  const row = (event.currentTarget as HTMLElement).closest('li')
  if (!row || typeof event.dataTransfer.setDragImage !== 'function') return
  const rect = row.getBoundingClientRect()
  event.dataTransfer.setDragImage(row, event.clientX - rect.left, event.clientY - rect.top)
}

export const useStoryDrag = <P>(options: UseStoryDragOptions<P>): UseStoryDragResult<P> => {
  const { mimeType, encode, decode: decodePayload } = options
  const [draggedNumber, setDraggedNumber] = useState<number | null>(null)
  const [hoveredNumber, setHoveredNumber] = useState<number | null>(null)

  const startDrag = useCallback(
    (event: React.DragEvent<HTMLElement>, storyNumber: number, payload: P): void => {
      event.dataTransfer.setData(mimeType, encode(payload))
      event.dataTransfer.effectAllowed = 'move'
      setDragImageFromClosestLi(event)
      setDraggedNumber(storyNumber)
    },
    [encode, mimeType]
  )

  const endDrag = useCallback((): void => {
    setDraggedNumber(null)
    setHoveredNumber(null)
  }, [])

  const hover = useCallback((storyNumber: number | null): void => {
    setHoveredNumber(storyNumber)
  }, [])

  const decode = useCallback(
    (event: React.DragEvent<HTMLElement>): P | null => {
      const raw = event.dataTransfer.getData(mimeType)
      if (!raw) return null
      return decodePayload(raw)
    },
    [decodePayload, mimeType]
  )

  const hasType = useCallback(
    (event: React.DragEvent<HTMLElement>): boolean => event.dataTransfer.types.includes(mimeType),
    [mimeType]
  )

  return { draggedNumber, hoveredNumber, startDrag, endDrag, hover, decode, hasType }
}
