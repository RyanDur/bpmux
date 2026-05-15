export const DEFAULT_FIRST_POSITION = 100
export const DEFAULT_STEP = 100

export interface PositionFor {
  position: number
  cascadeNeeded: boolean
}

export const computeInsertPosition = (
  insertIndex: number,
  existingPositions: number[]
): PositionFor => {
  const positions = [...existingPositions].sort((a, b) => a - b)
  if (positions.length === 0) {
    return { position: DEFAULT_FIRST_POSITION, cascadeNeeded: false }
  }
  if (insertIndex <= 0) {
    const first = positions[0]
    if (first > 1) {
      const midpoint = Math.floor(first / 2)
      return { position: midpoint, cascadeNeeded: midpoint <= 0 }
    }
    return { position: 0, cascadeNeeded: true }
  }
  if (insertIndex >= positions.length) {
    const last = positions[positions.length - 1]
    return { position: last + DEFAULT_STEP, cascadeNeeded: false }
  }
  const left = positions[insertIndex - 1]
  const right = positions[insertIndex]
  const gap = right - left
  if (gap >= 2) {
    return { position: left + Math.floor(gap / 2), cascadeNeeded: false }
  }
  return { position: left, cascadeNeeded: true }
}
