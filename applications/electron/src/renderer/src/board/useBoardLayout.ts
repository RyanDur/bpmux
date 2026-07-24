import { useCallback, useState } from 'react'
import {
  BOARD_DEFAULT_WIDTH,
  clampBoardWidth,
  computeCollapseToggle
} from '../board/BoardResizeHandle.utils'
import { useBoardWidthStorage } from '../board/useBoardWidthStorage'
import { RENDERER_IDENTITY } from '../state'

export type UseBoardLayoutResult = {
  boardOpen: boolean
  toggleBoardOpen: () => void
  width: number
  setBoardWidth: (next: number) => void
  toggleBoardCollapse: (viewportWidth: number) => void
}

export const useBoardLayout = (): UseBoardLayoutResult => {
  const [boardOpen, setBoardOpen] = useState<boolean>(true)
  const boardWidthStorage = useBoardWidthStorage(
    'bpmux-board-width',
    RENDERER_IDENTITY,
    BOARD_DEFAULT_WIDTH
  )
  const [restoreWidth, setRestoreWidth] = useState<number>(boardWidthStorage.width)

  const setBoardWidth = (next: number): void => {
    if (typeof window === 'undefined') {
      boardWidthStorage.setWidth(next)
      return
    }
    boardWidthStorage.setWidth(clampBoardWidth(next, window.innerWidth))
  }

  const toggleBoardCollapse = (viewportWidth: number): void => {
    const { nextWidth, nextRestoreWidth } = computeCollapseToggle(
      boardWidthStorage.width,
      restoreWidth,
      viewportWidth
    )
    setRestoreWidth(nextRestoreWidth)
    boardWidthStorage.setWidth(nextWidth)
  }

  const toggleBoardOpen = useCallback((): void => setBoardOpen((prev) => !prev), [])

  return {
    boardOpen,
    toggleBoardOpen,
    width: boardWidthStorage.width,
    setBoardWidth,
    toggleBoardCollapse
  }
}
