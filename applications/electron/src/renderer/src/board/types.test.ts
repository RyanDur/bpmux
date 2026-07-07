import { describe, expect, test } from 'vitest'
import {
  columnFor,
  columnForIn,
  groupOrderFrom,
  groupOrderOrFallback,
  GROUP_ORDER,
  type BoardColumn,
  type StoryState
} from './types'

describe('#514 columnFor — closed-state-aware bucketing', () => {
  describe('open issues route to active-pipeline columns (no regression vs pre-#514)', () => {
    test.each<[StoryState, ReturnType<typeof columnFor>]>([
      ['icebox', 'icebox'],
      ['ready', 'ready'],
      ['in-progress', 'in-progress'],
      ['desk-check', 'desk-check'],
      ['acceptance', 'acceptance'],
      ['ready-for-accept', 'acceptance'],
      ['needs-fixes', 'in-progress'],
      ['rejected', 'ready'],
      ['accepted', 'accepted']
    ])('open + state=%s → column %s', (state, column) => {
      expect(columnFor(state, false)).toBe(column)
    })
  })

  describe('#1146 slice 2b — closed-non-Accepted projects to RETIRED column', () => {
    test.each<[StoryState, ReturnType<typeof columnFor>]>([
      ['icebox', 'retired'],
      ['ready', 'retired'],
      ['in-progress', 'retired'],
      ['desk-check', 'retired'],
      ['acceptance', 'retired'],
      ['ready-for-accept', 'retired'],
      ['needs-fixes', 'retired'],
      ['rejected', 'retired']
    ])('closed + state=%s (non-Accepted) → RETIRED (decision-not-to-pursue)', (state, column) => {
      expect(columnFor(state, true)).toBe(column)
    })

    test('closed + state=accepted stays in ACCEPTED (success-narrative preserved)', () => {
      expect(columnFor('accepted', true)).toBe('accepted')
    })
  })

  describe('isClosed defaults to false (back-compat for callers that pre-date #514)', () => {
    test('omitting isClosed argument behaves as open', () => {
      expect(columnFor('in-progress')).toBe('in-progress')
      expect(columnFor('rejected')).toBe('ready')
    })
  })

  describe('#514 slice 2 — needs-fixes splits from rejected for OPEN issues', () => {
    test('OPEN + needs-fixes is dev-iterating-on-feedback → IN PROGRESS, not READY', () => {
      expect(columnFor('needs-fixes', false)).toBe('in-progress')
    })
    test('OPEN + rejected stays in READY (owner-reject = re-pickup-able)', () => {
      expect(columnFor('rejected', false)).toBe('ready')
    })
  })

  describe('#1300 Slice C — closed iteration markers stay in icebox as historical artifact', () => {
    test('closed iteration in state=icebox stays in ICEBOX column (closed-state visual record)', () => {
      expect(columnFor('icebox', true, 'iteration')).toBe('icebox')
    })
    test('closed iteration in any non-accepted state stays in ICEBOX (closed-marker invariant)', () => {
      expect(columnFor('ready', true, 'iteration')).toBe('icebox')
      expect(columnFor('in-progress', true, 'iteration')).toBe('icebox')
    })
    test('closed non-iteration stays routing to RETIRED (no regression)', () => {
      expect(columnFor('icebox', true, 'feature')).toBe('retired')
      expect(columnFor('icebox', true, 'epic')).toBe('retired')
      expect(columnFor('icebox', true)).toBe('retired')
    })
  })
})

describe('CM-2 columnForIn — bucketing against the live middle stage list', () => {
  const middle = ['build', 'review', 'harden']
  const bucket = columnForIn(middle)

  test('a state matching a configured middle stage buckets to itself', () => {
    expect(bucket('review')).toBe('review')
  })

  test('a truly unknown state buckets to the FIRST middle column (visible beats vanished)', () => {
    expect(bucket('mystery-stage')).toBe('build')
  })

  test('legacy needs-fixes buckets to the FIRST middle column', () => {
    expect(bucket('needs-fixes')).toBe('build')
  })

  test('legacy rejected buckets to ready', () => {
    expect(bucket('rejected')).toBe('ready')
  })

  test('legacy ready-for-accept buckets to acceptance', () => {
    expect(bucket('ready-for-accept')).toBe('acceptance')
  })

  test('fixed poles bucket to themselves', () => {
    expect(bucket('ready')).toBe('ready')
    expect(bucket('acceptance')).toBe('acceptance')
    expect(bucket('accepted')).toBe('accepted')
    expect(bucket('icebox')).toBe('icebox')
  })

  test('closed iteration marker stays in icebox regardless of its middle stage', () => {
    expect(bucket('review', true, 'iteration')).toBe('icebox')
  })

  test('closed non-accepted middle-stage story projects to retired', () => {
    expect(bucket('review', true, 'feature')).toBe('retired')
  })

  test('compat columnFor mirrors the two-stage default middle', () => {
    expect(columnFor('needs-fixes')).toBe('in-progress')
    expect(columnFor('mystery-stage')).toBe('in-progress')
  })
})

describe('CM-2 groupOrderFrom — wire columns map to display order', () => {
  const columns: BoardColumn[] = [
    { id: 'icebox', label: 'Icebox' },
    { id: 'ready', label: 'Ready' },
    { id: 'build', label: 'Build' },
    { id: 'review', label: 'Review' },
    { id: 'harden', label: 'Harden' },
    { id: 'acceptance', label: 'Acceptance' },
    { id: 'accepted', label: 'Accepted' },
    { id: 'retired', label: 'Retired' }
  ]

  test('orders ready, middle (config order), acceptance, accepted, retired, then icebox LAST', () => {
    expect(groupOrderFrom(columns).map((g) => g.state)).toEqual([
      'ready',
      'build',
      'review',
      'harden',
      'acceptance',
      'accepted',
      'retired',
      'icebox'
    ])
  })

  test('collapses accepted and retired; leaves ready/middle/acceptance/icebox open', () => {
    const collapsedByState = new Map(groupOrderFrom(columns).map((g) => [g.state, g.collapsed]))
    expect(collapsedByState.get('accepted')).toBe(true)
    expect(collapsedByState.get('retired')).toBe(true)
    expect(collapsedByState.get('review')).toBe(false)
    expect(collapsedByState.get('ready')).toBe(false)
    expect(collapsedByState.get('icebox')).toBe(false)
  })

  test('carries the wire labels through to the group spec', () => {
    expect(groupOrderFrom(columns).find((g) => g.state === 'review')?.label).toBe('Review')
  })
})

describe('CM-2 groupOrderOrFallback — absent wire columns keep the legacy board', () => {
  test('no wire columns falls back to the legacy GROUP_ORDER', () => {
    expect(groupOrderOrFallback([])).toBe(GROUP_ORDER)
  })

  test('present wire columns are used in place of the fallback', () => {
    expect(groupOrderOrFallback([{ id: 'ready', label: 'Ready' }]).map((g) => g.state)).toEqual([
      'ready'
    ])
  })
})
