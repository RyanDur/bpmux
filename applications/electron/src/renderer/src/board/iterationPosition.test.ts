import { describe, expect, it } from 'vitest'

import { computeInsertPosition, DEFAULT_FIRST_POSITION } from './iterationPosition'

describe('computeInsertPosition — sparse-numeric position-suffix for iteration-{Name}.{N} labels', () => {
  it('returns the default first position when the list is empty', () => {
    const result = computeInsertPosition(0, [])
    expect(result.position).toBe(DEFAULT_FIRST_POSITION)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('inserts before the only existing position via halving', () => {
    const result = computeInsertPosition(0, [100])
    expect(result.position).toBe(50)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('inserts after the only existing position by stepping forward', () => {
    const result = computeInsertPosition(1, [100])
    expect(result.position).toBe(200)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('inserts between two adjacent positions at the midpoint', () => {
    const result = computeInsertPosition(1, [100, 200])
    expect(result.position).toBe(150)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('inserts before the first position via halving when first > 1', () => {
    const result = computeInsertPosition(0, [100, 200])
    expect(result.position).toBe(50)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('inserts after the last position by stepping by DEFAULT_STEP', () => {
    const result = computeInsertPosition(2, [100, 200])
    expect(result.position).toBe(300)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('flags cascadeNeeded when the gap between neighbors is exhausted', () => {
    const result = computeInsertPosition(1, [100, 101])
    expect(result.cascadeNeeded).toBe(true)
  })

  it('flags cascadeNeeded when inserting before position 1 leaves no room', () => {
    const result = computeInsertPosition(0, [1])
    expect(result.cascadeNeeded).toBe(true)
  })

  it('handles unsorted input by sorting first', () => {
    const result = computeInsertPosition(1, [300, 100, 200])
    expect(result.position).toBe(150)
    expect(result.cascadeNeeded).toBe(false)
  })

  it('returns midpoint as integer (Math.floor) on odd-sum gap', () => {
    const result = computeInsertPosition(1, [100, 105])
    expect(result.position).toBe(102)
    expect(result.cascadeNeeded).toBe(false)
  })
})
