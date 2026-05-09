import { describe, expect, test } from 'vitest'
import { applySelfUpdateRestartPolicy } from './selfUpdateRestartPolicy'

describe('applySelfUpdateRestartPolicy — #118 Electron-side self-update loop guard', () => {
  test('first restart in an empty window returns respawn', () => {
    const recent: number[] = []
    const decision = applySelfUpdateRestartPolicy(recent, 1_000, {
      windowMs: 60_000,
      maxRestarts: 5
    })
    expect(decision).toEqual({ action: 'respawn', restartsInWindow: 1 })
    expect(recent).toEqual([1_000])
  })

  test('respawns through max-1 restarts within window', () => {
    const recent: number[] = []
    for (let i = 1; i < 5; i++) {
      const decision = applySelfUpdateRestartPolicy(recent, i * 1_000, {
        windowMs: 60_000,
        maxRestarts: 5
      })
      expect(decision.action).toBe('respawn')
      expect(decision.restartsInWindow).toBe(i)
    }
  })

  test('halts on the Nth restart inside the window (5-in-60s default)', () => {
    const recent: number[] = []
    for (let i = 1; i <= 4; i++) {
      applySelfUpdateRestartPolicy(recent, i * 1_000, {
        windowMs: 60_000,
        maxRestarts: 5
      })
    }
    const fifth = applySelfUpdateRestartPolicy(recent, 5_000, {
      windowMs: 60_000,
      maxRestarts: 5
    })
    expect(fifth).toEqual({ action: 'halt', restartsInWindow: 5 })
  })

  test('prunes restarts older than the window before counting', () => {
    const recent: number[] = [1_000, 2_000, 3_000, 4_000]
    const now = 4_000 + 60_001
    const decision = applySelfUpdateRestartPolicy(recent, now, {
      windowMs: 60_000,
      maxRestarts: 5
    })
    expect(decision).toEqual({ action: 'respawn', restartsInWindow: 1 })
    expect(recent).toEqual([now])
  })

  test('partial-prune: stale entries fall out of window while fresh stay', () => {
    const recent: number[] = [1_000, 2_000, 70_000, 71_000]
    const decision = applySelfUpdateRestartPolicy(recent, 75_000, {
      windowMs: 60_000,
      maxRestarts: 5
    })
    expect(decision).toEqual({ action: 'respawn', restartsInWindow: 3 })
    expect(recent).toEqual([70_000, 71_000, 75_000])
  })
})
