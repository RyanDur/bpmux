import { describe, expect, test } from 'vitest'
import { dropIndexForRowDrag } from './boardDropIndex'

const ROW_TOP = 100
const ROW_HEIGHT = 40
const NEAR_TOP = ROW_TOP + 5
const NEAR_BOTTOM = ROW_TOP + ROW_HEIGHT - 5

describe('#1524 dropIndexForRowDrag — a single drag onto the immediate neighbor swaps', () => {
  test('immediate row BELOW swaps on the whole row (near-half too), not only its far half', () => {
    expect(
      dropIndexForRowDrag({
        fromIndex: 0,
        hoveredIndex: 1,
        clientY: NEAR_TOP,
        rectTop: ROW_TOP,
        rectHeight: ROW_HEIGHT
      })
    ).toBe(2)
  })

  test('immediate row ABOVE swaps on the whole row (near-half too), not only its far half', () => {
    expect(
      dropIndexForRowDrag({
        fromIndex: 2,
        hoveredIndex: 1,
        clientY: NEAR_BOTTOM,
        rectTop: ROW_TOP,
        rectHeight: ROW_HEIGHT
      })
    ).toBe(1)
  })

  test('non-adjacent row keeps midpoint precision — upper half inserts before', () => {
    expect(
      dropIndexForRowDrag({
        fromIndex: 0,
        hoveredIndex: 3,
        clientY: NEAR_TOP,
        rectTop: ROW_TOP,
        rectHeight: ROW_HEIGHT
      })
    ).toBe(3)
  })

  test('non-adjacent row keeps midpoint precision — lower half inserts after', () => {
    expect(
      dropIndexForRowDrag({
        fromIndex: 0,
        hoveredIndex: 3,
        clientY: NEAR_BOTTOM,
        rectTop: ROW_TOP,
        rectHeight: ROW_HEIGHT
      })
    ).toBe(4)
  })

  test('unknown source falls back to the midpoint rule', () => {
    expect(
      dropIndexForRowDrag({
        fromIndex: null,
        hoveredIndex: 1,
        clientY: NEAR_TOP,
        rectTop: ROW_TOP,
        rectHeight: ROW_HEIGHT
      })
    ).toBe(1)
  })
})
