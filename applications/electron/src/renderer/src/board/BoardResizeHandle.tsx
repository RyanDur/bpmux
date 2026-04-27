import { useCallback, useEffect, useRef, useState } from 'react'
import './BoardResizeHandle.css'

export const BOARD_MIN_WIDTH = 260
export const BOARD_MAX_WIDTH_ABS = 800
export const BOARD_MAX_WIDTH_VP_FRAC = 0.5
export const BOARD_DEFAULT_WIDTH = 360
export const BOARD_RESIZE_KEYBOARD_STEP = 24

const computeMaxWidth = (viewportWidth: number): number =>
  Math.min(Math.floor(viewportWidth * BOARD_MAX_WIDTH_VP_FRAC), BOARD_MAX_WIDTH_ABS)

export const clampBoardWidth = (raw: number, viewportWidth: number): number => {
  const max = computeMaxWidth(viewportWidth)
  if (raw < BOARD_MIN_WIDTH) return BOARD_MIN_WIDTH
  if (raw > max) return max
  return raw
}

export const widthPercent = (width: number, viewportWidth: number): number => {
  const max = computeMaxWidth(viewportWidth)
  if (max <= BOARD_MIN_WIDTH) return 0
  const fraction = (width - BOARD_MIN_WIDTH) / (max - BOARD_MIN_WIDTH)
  if (fraction < 0) return 0
  if (fraction > 1) return 100
  return Math.round(fraction * 100)
}

interface Props {
  width: number
  setWidth: (next: number) => void
  controlsId: string
}

const BoardResizeHandle = ({ width, setWidth, controlsId }: Props): React.JSX.Element => {
  const [viewportWidth, setViewportWidth] = useState<number>(() =>
    typeof window === 'undefined' ? 1024 : window.innerWidth
  )
  const lastWidthBeforeCollapse = useRef<number>(width)

  useEffect(() => {
    if (typeof window === 'undefined') return
    const onResize = (): void => setViewportWidth(window.innerWidth)
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  const onPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault()
      const startX = e.clientX
      const startWidth = width
      const handle = e.currentTarget
      const aside = document.getElementById(controlsId)
      handle.setPointerCapture(e.pointerId)
      let latest = startWidth

      const onMove = (ev: PointerEvent): void => {
        const dx = startX - ev.clientX
        latest = clampBoardWidth(startWidth + dx, window.innerWidth)
        if (aside) aside.style.flex = `0 0 ${latest}px`
        handle.setAttribute('aria-valuenow', String(widthPercent(latest, window.innerWidth)))
      }
      const onUp = (ev: PointerEvent): void => {
        handle.releasePointerCapture(ev.pointerId)
        handle.removeEventListener('pointermove', onMove)
        handle.removeEventListener('pointerup', onUp)
        handle.removeEventListener('pointercancel', onUp)
        setWidth(latest)
      }
      handle.addEventListener('pointermove', onMove)
      handle.addEventListener('pointerup', onUp)
      handle.addEventListener('pointercancel', onUp)
    },
    [controlsId, setWidth, width]
  )

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      const max = computeMaxWidth(viewportWidth)
      if (e.key === 'ArrowLeft') {
        e.preventDefault()
        setWidth(clampBoardWidth(width - BOARD_RESIZE_KEYBOARD_STEP, viewportWidth))
        return
      }
      if (e.key === 'ArrowRight') {
        e.preventDefault()
        setWidth(clampBoardWidth(width + BOARD_RESIZE_KEYBOARD_STEP, viewportWidth))
        return
      }
      if (e.key === 'Home') {
        e.preventDefault()
        setWidth(BOARD_MIN_WIDTH)
        return
      }
      if (e.key === 'End') {
        e.preventDefault()
        setWidth(max)
        return
      }
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault()
        if (width > BOARD_MIN_WIDTH) {
          lastWidthBeforeCollapse.current = width
          setWidth(BOARD_MIN_WIDTH)
        } else {
          setWidth(clampBoardWidth(lastWidthBeforeCollapse.current, viewportWidth))
        }
        return
      }
    },
    [setWidth, viewportWidth, width]
  )

  return (
    <div
      className="board-resize-handle"
      data-test="board-resize-handle"
      role="separator"
      aria-orientation="vertical"
      aria-controls={controlsId}
      aria-label="Board panel width"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={widthPercent(width, viewportWidth)}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onKeyDown={onKeyDown}
    />
  )
}

export default BoardResizeHandle
