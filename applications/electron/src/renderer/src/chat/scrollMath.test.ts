import { describe, expect, test } from 'vitest'
import { isNearTopUnderColumnReverse, NEAR_TOP_THRESHOLD_PX } from './scrollMath'

const SCROLL_HEIGHT = 5000
const CLIENT_HEIGHT = 600
const MAX_SCROLL = SCROLL_HEIGHT - CLIENT_HEIGHT

describe('isNearTopUnderColumnReverse', () => {
  test('returns false at visual bottom', () => {
    expect(isNearTopUnderColumnReverse(0, SCROLL_HEIGHT, CLIENT_HEIGHT)).toBe(false)
  })

  test('returns false in the middle of the list', () => {
    expect(isNearTopUnderColumnReverse(MAX_SCROLL / 2, SCROLL_HEIGHT, CLIENT_HEIGHT)).toBe(false)
  })

  test('returns true at visual top', () => {
    expect(isNearTopUnderColumnReverse(MAX_SCROLL, SCROLL_HEIGHT, CLIENT_HEIGHT)).toBe(true)
  })

  test('returns true within threshold of visual top', () => {
    expect(
      isNearTopUnderColumnReverse(
        MAX_SCROLL - (NEAR_TOP_THRESHOLD_PX - 50),
        SCROLL_HEIGHT,
        CLIENT_HEIGHT
      )
    ).toBe(true)
  })

  test('returns false just outside threshold', () => {
    expect(
      isNearTopUnderColumnReverse(
        MAX_SCROLL - (NEAR_TOP_THRESHOLD_PX + 1),
        SCROLL_HEIGHT,
        CLIENT_HEIGHT
      )
    ).toBe(false)
  })

  test('handles negative scrollTop (Firefox column-reverse)', () => {
    expect(isNearTopUnderColumnReverse(-MAX_SCROLL, SCROLL_HEIGHT, CLIENT_HEIGHT)).toBe(true)
  })

  test('returns false when content does not overflow', () => {
    expect(isNearTopUnderColumnReverse(0, 500, 600)).toBe(false)
    expect(isNearTopUnderColumnReverse(0, 600, 600)).toBe(false)
  })

  test('threshold parameter is overridable', () => {
    const scrollTop = MAX_SCROLL - 300
    expect(isNearTopUnderColumnReverse(scrollTop, SCROLL_HEIGHT, CLIENT_HEIGHT)).toBe(false)
    expect(isNearTopUnderColumnReverse(scrollTop, SCROLL_HEIGHT, CLIENT_HEIGHT, 400)).toBe(true)
  })

  test('NEAR_TOP_THRESHOLD_PX default is the designer-tuned value', () => {
    expect(NEAR_TOP_THRESHOLD_PX).toBe(200)
  })
})
