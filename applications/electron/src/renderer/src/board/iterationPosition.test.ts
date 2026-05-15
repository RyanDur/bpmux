import { describe, expect, it } from 'vitest'

import {
  computeDropMutation,
  computeInsertPosition,
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

describe('computeDropMutation — drag-drop yields canonical add/remove label-pair', () => {
  it('within-iteration reorder produces single-iteration add/remove pair', () => {
    const result = computeDropMutation({
      draggedNumber: 7411,
      draggedThemeName: 'Cassiopeia',
      draggedOldPosition: 300,
      targetThemeName: 'Cassiopeia',
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: true
    })
    expect(result).toEqual({
      add: ['iteration-Cassiopeia.50'],
      remove: ['iteration-Cassiopeia.300']
    })
  })

  it('within-iteration drop on own position returns null (no-op)', () => {
    const result = computeDropMutation({
      draggedNumber: 7411,
      draggedThemeName: 'Cassiopeia',
      draggedOldPosition: 200,
      targetThemeName: 'Cassiopeia',
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 1,
      targetMembersIncludeDragged: true
    })
    expect(result).toBeNull()
  })

  it('cross-iteration drop retags with new iteration prefix', () => {
    const result = computeDropMutation({
      draggedNumber: 7411,
      draggedThemeName: 'Cassiopeia',
      draggedOldPosition: 200,
      targetThemeName: 'Pegasus',
      targetMemberPositions: [100, 200],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: false
    })
    expect(result).toEqual({
      add: ['iteration-Pegasus.50'],
      remove: ['iteration-Cassiopeia.200']
    })
  })

  it('cross-iteration drop into empty container places at default-first', () => {
    const result = computeDropMutation({
      draggedNumber: 7411,
      draggedThemeName: 'Cassiopeia',
      draggedOldPosition: 200,
      targetThemeName: 'Pegasus',
      targetMemberPositions: [],
      targetMemberIndex: 0,
      targetMembersIncludeDragged: false
    })
    expect(result).toEqual({
      add: ['iteration-Pegasus.100'],
      remove: ['iteration-Cassiopeia.200']
    })
  })

  it('within-iteration drop after last position steps forward', () => {
    const result = computeDropMutation({
      draggedNumber: 7411,
      draggedThemeName: 'Cassiopeia',
      draggedOldPosition: 100,
      targetThemeName: 'Cassiopeia',
      targetMemberPositions: [100, 200, 300],
      targetMemberIndex: 2,
      targetMembersIncludeDragged: true
    })
    expect(result?.add).toEqual(['iteration-Cassiopeia.250'])
    expect(result?.remove).toEqual(['iteration-Cassiopeia.100'])
  })
})
