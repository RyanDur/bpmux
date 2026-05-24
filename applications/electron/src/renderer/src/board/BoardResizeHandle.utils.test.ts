import { describe, expect, test } from 'vitest'
import { BOARD_MIN_WIDTH, clampBoardWidth, computeCollapseToggle } from './BoardResizeHandle.utils'

describe('#1538 computeCollapseToggle — collapse/expand width state-machine (restore-memory at the width-owner)', () => {
  const VIEWPORT = 1440

  test('collapse: when width is above min, returns min and remembers the pre-collapse width', () => {
    const result = computeCollapseToggle(420, 0, VIEWPORT)
    expect(result.nextWidth).toBe(BOARD_MIN_WIDTH)
    expect(result.nextRestoreWidth).toBe(420)
  })

  test('expand: when width is at min, restores the remembered width (clamped) and keeps the memory', () => {
    const result = computeCollapseToggle(BOARD_MIN_WIDTH, 420, VIEWPORT)
    expect(result.nextWidth).toBe(clampBoardWidth(420, VIEWPORT))
    expect(result.nextRestoreWidth).toBe(420)
  })

  test('collapse→expand round-trips to the exact pre-collapse width', () => {
    const collapsed = computeCollapseToggle(500, 0, VIEWPORT)
    const expanded = computeCollapseToggle(
      collapsed.nextWidth,
      collapsed.nextRestoreWidth,
      VIEWPORT
    )
    expect(expanded.nextWidth).toBe(clampBoardWidth(500, VIEWPORT))
  })

  test('expand clamps a remembered width that exceeds the current viewport max', () => {
    const narrowViewport = 700
    const result = computeCollapseToggle(BOARD_MIN_WIDTH, 9999, narrowViewport)
    expect(result.nextWidth).toBe(clampBoardWidth(9999, narrowViewport))
  })
})
