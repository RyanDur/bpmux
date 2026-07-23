import { describe, expect, test } from 'vitest'
import {
  columnFor,
  columnForIn,
  groupOrderFrom,
  groupOrderOrFallback,
  isConcluded,
  GROUP_ORDER,
  type BoardColumn,
  type StoryState
} from './types'

describe('columnFor — state-derived bucketing', () => {
  test.each<[StoryState, ReturnType<typeof columnFor>]>([
    ['icebox', 'icebox'],
    ['ready', 'ready'],
    ['in-progress', 'in-progress'],
    ['desk-check', 'desk-check'],
    ['acceptance', 'acceptance'],
    ['ready-for-accept', 'acceptance'],
    ['needs-fixes', 'in-progress'],
    ['rejected', 'ready'],
    ['accepted', 'accepted'],
    ['retired', 'retired']
  ])('state=%s → column %s', (state, column) => {
    expect(columnFor(state)).toBe(column)
  })

  test('a concluded state self-buckets to its own pole (accepted→accepted, retired→retired)', () => {
    expect(columnFor('accepted')).toBe('accepted')
    expect(columnFor('retired')).toBe('retired')
  })

  test('a truly unknown state buckets to the first middle column', () => {
    expect(columnFor('mystery-stage')).toBe('in-progress')
  })
})

describe('isConcluded — closedness derives from the workflow state', () => {
  test('accepted and retired are concluded; every active state is not', () => {
    expect(isConcluded('accepted')).toBe(true)
    expect(isConcluded('retired')).toBe(true)
    expect(isConcluded('icebox')).toBe(false)
    expect(isConcluded('ready')).toBe(false)
    expect(isConcluded('in-progress')).toBe(false)
    expect(isConcluded('acceptance')).toBe(false)
  })
})

describe('columnForIn — bucketing against the live middle stage list', () => {
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
    expect(bucket('retired')).toBe('retired')
    expect(bucket('icebox')).toBe('icebox')
  })

  test('compat columnFor mirrors the two-stage default middle', () => {
    expect(columnFor('needs-fixes')).toBe('in-progress')
    expect(columnFor('mystery-stage')).toBe('in-progress')
  })
})

describe('groupOrderFrom — wire columns map to display order', () => {
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

describe('groupOrderOrFallback — absent wire columns keep the legacy board', () => {
  test('no wire columns falls back to the legacy GROUP_ORDER', () => {
    expect(groupOrderOrFallback([])).toBe(GROUP_ORDER)
  })

  test('present wire columns are used in place of the fallback', () => {
    expect(groupOrderOrFallback([{ id: 'ready', label: 'Ready' }]).map((g) => g.state)).toEqual([
      'ready'
    ])
  })
})
