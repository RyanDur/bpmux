/**
 * Under column-reverse layout (MessageList #363), `scrollTop=0` is the
 * visual BOTTOM (newest); `|scrollTop|` grows toward
 * `scrollHeight - clientHeight` at the visual TOP. Firefox negates
 * scrollTop under column-reverse — `Math.abs` normalizes engines.
 *
 * `NEAR_TOP_THRESHOLD_PX` fires BEFORE the user reaches the top so the
 * next paged-history page lands without a scroll-jump.
 */

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
