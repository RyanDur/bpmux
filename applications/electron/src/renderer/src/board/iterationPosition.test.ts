import { describe, expect, it } from 'vitest'

import {
  computeEstablishOrderMutations,
  computeInsertPosition,
  computeIterationOrderMutation,
  computeMemberDropMutation,
  DEFAULT_FIRST_POSITION
} from './iterationPosition'

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

describe('computeMemberDropMutation — canonical identity add/remove label-pair', () => {
  it('within-iteration reorder produces single-iteration add/remove pair', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 300,
      targetIterationNumber: 1464,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: true
    })
    expect(result).toEqual({
      add: ['parent-iteration-#1464.50'],
      remove: ['parent-iteration-#1464.300']
    })
  })

  it('within-iteration drop on own position returns null (no-op)', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 200,
      targetIterationNumber: 1464,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 1,
      targetMembersIncludeDragged: true
    })
    expect(result).toBeNull()
  })

  it('cross-iteration drop retags with the target iteration identity', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 200,
      targetIterationNumber: 1491,
      targetMemberPositions: [100, 200],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: false
    })
    expect(result).toEqual({
      add: ['parent-iteration-#1491.50'],
      remove: ['parent-iteration-#1464.200']
    })
  })

  it('cross-iteration drop into empty container places at default-first', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 200,
      targetIterationNumber: 1491,
      targetMemberPositions: [],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: false
    })
    expect(result).toEqual({
      add: ['parent-iteration-#1491.100'],
      remove: ['parent-iteration-#1464.200']
    })
  })

  it('within-iteration drop on last places source AT last (after stepping past last)', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 100,
      targetIterationNumber: 1464,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 2,
      targetMembersIncludeDragged: true
    })
    expect(result?.add).toEqual(['parent-iteration-#1464.400'])
    expect(result?.remove).toEqual(['parent-iteration-#1464.100'])
  })

  it('within-iteration adjacent-downward swap produces non-null mutation between target and next', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 100,
      targetIterationNumber: 1464,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 1,
      targetMembersIncludeDragged: true
    })
    expect(result?.add).toEqual(['parent-iteration-#1464.250'])
    expect(result?.remove).toEqual(['parent-iteration-#1464.100'])
  })

  it('within-iteration adjacent-upward swap places source before target', () => {
    const result = computeMemberDropMutation({
      draggedIterationNumber: 1464,
      draggedOldPosition: 200,
      targetIterationNumber: 1464,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: true
    })
    expect(result?.add).toEqual(['parent-iteration-#1464.50'])
    expect(result?.remove).toEqual(['parent-iteration-#1464.200'])
  })
})

describe('computeIterationOrderMutation — iteration-order.N reorder label-pair', () => {
  it('reorder produces an iteration-order add/remove pair', () => {
    const result = computeIterationOrderMutation({
      draggedOldPosition: 300,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: true
    })
    expect(result).toEqual({
      add: ['iteration-order.50'],
      remove: ['iteration-order.300']
    })
  })

  it('drop on own position returns null (no-op)', () => {
    const result = computeIterationOrderMutation({
      draggedOldPosition: 200,
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 1,
      targetMembersIncludeDragged: true
    })
    expect(result).toBeNull()
  })
})

describe('computeEstablishOrderMutations — self-heal a fully/partially unordered iteration set', () => {
  it('assigns clean spaced positions to a set with no order labels at all', () => {
    const result = computeEstablishOrderMutations([
      { number: 9701, currentPosition: null },
      { number: 9702, currentPosition: null }
    ])
    expect(result).toEqual([
      { iterationNumber: 9701, add: ['iteration-order.100'], remove: [] },
      { iterationNumber: 9702, add: ['iteration-order.200'], remove: [] }
    ])
  })

  it('emits a mutation only for an iteration whose position actually changes, removing its stale label', () => {
    const result = computeEstablishOrderMutations([
      { number: 9701, currentPosition: 300 },
      { number: 9702, currentPosition: null }
    ])
    expect(result).toEqual([
      { iterationNumber: 9701, add: ['iteration-order.100'], remove: ['iteration-order.300'] },
      { iterationNumber: 9702, add: ['iteration-order.200'], remove: [] }
    ])
  })

  it('emits nothing when the set is already cleanly spaced in the target order', () => {
    const result = computeEstablishOrderMutations([
      { number: 9701, currentPosition: 100 },
      { number: 9702, currentPosition: 200 }
    ])
    expect(result).toEqual([])
  })
})
