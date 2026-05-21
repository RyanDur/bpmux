import { useCallback, useEffect, useRef, useState } from 'react'
import {
  BOARD_MIN_WIDTH,
  BOARD_RESIZE_KEYBOARD_STEP,
  clampBoardWidth,
  computeMaxWidth,
  widthPercent
} from './BoardResizeHandle.utils'
import './BoardResizeHandle.css'

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
      role="slider"
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
