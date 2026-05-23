export const NEAR_TOP_THRESHOLD_PX = 200

export const isNearTopUnderColumnReverse = (
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  threshold: number = NEAR_TOP_THRESHOLD_PX
): boolean => {
  const maxScroll = scrollHeight - clientHeight
  if (maxScroll <= 0) return false
  return Math.abs(scrollTop) >= maxScroll - threshold
}
