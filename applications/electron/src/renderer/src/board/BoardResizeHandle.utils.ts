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

export { computeMaxWidth }
