import { describe, expect, test } from 'vitest'
import { stateAriaLabel, stateGlyph, type SyncState } from './syncState'

const ALL_STATES: SyncState[] = [
  'connected',
  'loading',
  'syncing',
  'offline-queued',
  'drained-permanent-failure',
  'stale',
  'startup-fail'
]

describe('#1636 stateGlyph — each state maps to its own distinct glyph', () => {
  const cases: Array<[SyncState, string]> = [
    ['connected', '●'],
    ['loading', '○'],
    ['syncing', '◌'],
    ['offline-queued', '⚠'],
    ['drained-permanent-failure', '✗'],
    ['stale', '◔'],
    ['startup-fail', '⊘']
  ]

  test.each(cases)('%s → %s', (state, glyph) => {
    expect(stateGlyph(state)).toBe(glyph)
  })

  test('only connected glances as the filled ● (the false-green guard)', () => {
    for (const state of ALL_STATES) {
      if (state !== 'connected') expect(stateGlyph(state)).not.toBe('●')
    }
  })
})

describe('#1636 stateAriaLabel — each state announces its true condition', () => {
  const fixedCases: Array<[SyncState, string]> = [
    ['connected', 'Sync state: connected'],
    ['loading', 'Sync state: loading — awaiting first report'],
    ['stale', 'Sync state: stale; last successful sync exceeded the staleness threshold'],
    ['startup-fail', 'Sync state: startup pull failed; board is empty until GitHub sync succeeds']
  ]

  test.each(fixedCases)('%s → exact screen-reader string', (state, label) => {
    expect(stateAriaLabel(state, 0)).toBe(label)
  })

  test('count-bearing states interpolate the count into the announcement', () => {
    expect(stateAriaLabel('syncing', 3)).toBe('Sync state: syncing 3 pending')
    expect(stateAriaLabel('offline-queued', 2)).toBe('Sync state: offline, 2 mutations queued')
    expect(stateAriaLabel('drained-permanent-failure', 4)).toBe(
      'Sync state: 4 mutations dropped on permanent failure'
    )
  })

  test('only connected announces the all-clear "connected" (the false-green guard)', () => {
    for (const state of ALL_STATES) {
      if (state !== 'connected') expect(stateAriaLabel(state, 1)).not.toBe('Sync state: connected')
    }
  })
})
