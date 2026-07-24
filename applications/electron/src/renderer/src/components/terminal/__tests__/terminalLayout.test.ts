import { describe, expect, test } from 'vitest'
import { restoreTerminalLayout } from '../terminalLayout'

describe('restoreTerminalLayout — re-attach the live SET on refresh, never resurrect an exited one', () => {
  test('with no saved hint, the full live set restores in server order and the first is active', () => {
    const restored = restoreTerminalLayout(['a', 'b', 'c'], null)
    expect(restored.orderedIds).toEqual(['a', 'b', 'c'])
    expect(restored.activeId).toBe('a')
  })

  test('the saved order is honoured for the terminals that are still live', () => {
    const restored = restoreTerminalLayout(['a', 'b', 'c'], {
      order: ['c', 'a', 'b'],
      activeId: 'b'
    })
    expect(restored.orderedIds).toEqual(['c', 'a', 'b'])
    expect(restored.activeId).toBe('b')
  })

  test('a terminal in the hint but NOT in the live set (exited during disconnect) is dropped, never resurrected', () => {
    const restored = restoreTerminalLayout(['a', 'c'], { order: ['a', 'b', 'c'], activeId: 'a' })
    expect(restored.orderedIds).toEqual(['a', 'c'])
    expect(restored.orderedIds).not.toContain('b')
  })

  test('a live terminal absent from the hint (spawned elsewhere) is appended, so the FULL set restores', () => {
    const restored = restoreTerminalLayout(['a', 'b', 'z'], { order: ['b', 'a'], activeId: 'a' })
    expect(restored.orderedIds).toEqual(['b', 'a', 'z'])
  })

  test('when the hinted-active terminal has exited, active falls back to the first restored terminal', () => {
    const restored = restoreTerminalLayout(['a', 'b'], { order: ['a', 'b'], activeId: 'gone' })
    expect(restored.activeId).toBe('a')
  })

  test('an empty live set restores nothing and has no active terminal', () => {
    const restored = restoreTerminalLayout([], { order: ['a', 'b'], activeId: 'a' })
    expect(restored.orderedIds).toEqual([])
    expect(restored.activeId).toBeNull()
  })
})
