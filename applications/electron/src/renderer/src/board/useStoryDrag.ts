import { useCallback, useState } from 'react'

export const DRAG_MIMETYPE_REGISTRY = {
  STORY_ROW: 'application/x-bpmux-story',
  ITERATION_MEMBER: 'application/x-bpmux-iteration-member',
  ITERATION_ITSELF: 'application/x-bpmux-iteration-itself',
  OUTCOME_ITERATION: 'application/x-bpmux-outcome-iteration'
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

const DRAG_GHOST_MARKER = 'data-drag-ghost'

const removeDragGhosts = (): void => {
  if (typeof document === 'undefined') return
  document.querySelectorAll(`[${DRAG_GHOST_MARKER}]`).forEach((el) => el.remove())
}

const copyPaintedStyles = (source: Element, clone: Element): void => {
  const sourceEls = [source, ...Array.from(source.querySelectorAll('*'))]
  const cloneEls = [clone, ...Array.from(clone.querySelectorAll('*'))]
  sourceEls.forEach((sourceEl, index) => {
    const cloneEl = cloneEls[index]
    if (cloneEl === undefined || !(cloneEl instanceof HTMLElement)) return
    const computed = window.getComputedStyle(sourceEl)
    cloneEl.style.backgroundColor = computed.backgroundColor
    cloneEl.style.color = computed.color
    cloneEl.style.borderColor = computed.borderColor
  })
}

const setDragImageFromClosestLi = (event: React.DragEvent<HTMLElement>): void => {
  const row = (event.currentTarget as HTMLElement).closest('li')
  if (!row || typeof event.dataTransfer.setDragImage !== 'function') return
  removeDragGhosts()
  const rect = row.getBoundingClientRect()
  const ghost = row.cloneNode(true) as HTMLElement
  if (typeof window.getComputedStyle === 'function') copyPaintedStyles(row, ghost)
  ghost.removeAttribute('data-test')
  ghost.removeAttribute('id')
  ghost.querySelectorAll('[data-test], [id]').forEach((el) => {
    el.removeAttribute('data-test')
    el.removeAttribute('id')
  })
  ghost.setAttribute(DRAG_GHOST_MARKER, '')
  ghost.setAttribute('aria-hidden', 'true')
  ghost.style.position = 'fixed'
  ghost.style.top = '-10000px'
  ghost.style.left = '0'
  ghost.style.width = `${rect.width}px`
  ghost.style.pointerEvents = 'none'
  document.body.appendChild(ghost)
  event.dataTransfer.setDragImage(ghost, event.clientX - rect.left, event.clientY - rect.top)
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
    removeDragGhosts()
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
