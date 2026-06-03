import { describe, expect, test } from 'vitest'
import { RECENCY_WINDOW_MS, orderGroupsByActivity, relativeTime } from './coordinationActivity'
import type { CoordinationPair } from './coordinationPairs'

const NOW = Date.parse('2026-06-03T12:00:00.000Z')
const at = (msAgo: number): string => new Date(NOW - msAgo).toISOString()

describe('#1605 relativeTime — low-ink recency label', () => {
  test('under a minute reads "now"', () => {
    expect(relativeTime(at(30_000), NOW)).toBe('now')
  })
  test('minutes', () => {
    expect(relativeTime(at(5 * 60_000), NOW)).toBe('5m')
  })
  test('hours', () => {
    expect(relativeTime(at(90 * 60_000), NOW)).toBe('1h')
  })
  test('days', () => {
    expect(relativeTime(at(50 * 60 * 60_000), NOW)).toBe('2d')
  })
})

describe('#1605 orderGroupsByActivity — recency-first, active-distinguished-from-idle', () => {
  const pair = (a: string, b: string, lastTs: string, count: number): CoordinationPair => ({
    agentA: a,
    agentB: b,
    lastTs,
    count
  })

  test('groups with coordination sort most-recent-first; idle groups sink below', () => {
    const groups = [
      ['dev', 'pm'],
      ['designer', 'pm'],
      ['designer', 'dev']
    ]
    const pairs = [pair('designer', 'dev', at(30_000), 1), pair('dev', 'pm', at(20 * 60_000), 5)]
    const ordered = orderGroupsByActivity(groups, pairs, NOW)
    expect(ordered.map((g) => g.names.join('+'))).toEqual(['designer+dev', 'dev+pm', 'designer+pm'])
    expect(ordered[2].activity).toBeNull()
  })

  test('a pair active within the recency window is flagged recent (gets the active-now dot)', () => {
    const ordered = orderGroupsByActivity([['dev', 'pm']], [pair('dev', 'pm', at(60_000), 1)], NOW)
    expect(ordered[0].activity?.recent).toBe(true)
  })

  test('a pair with coordination but stale (outside the window) is NOT recent — no false active-now dot', () => {
    const ordered = orderGroupsByActivity(
      [['dev', 'pm']],
      [pair('dev', 'pm', at(RECENCY_WINDOW_MS + 60_000), 9)],
      NOW
    )
    expect(ordered[0].activity).not.toBeNull()
    expect(ordered[0].activity?.recent).toBe(false)
  })

  test('the discrimination guard: an active pair and an idle pair render distinctly (dot+top vs null+sunk)', () => {
    const ordered = orderGroupsByActivity(
      [
        ['dev', 'pm'],
        ['designer', 'pm']
      ],
      [pair('dev', 'pm', at(30_000), 3)],
      NOW
    )
    expect(ordered[0].names.join('+')).toBe('dev+pm')
    expect(ordered[0].activity?.recent).toBe(true)
    expect(ordered[1].names.join('+')).toBe('designer+pm')
    expect(ordered[1].activity).toBeNull()
  })

  test('a 3+ member group never matches a 2-member pair (no spurious activity)', () => {
    const ordered = orderGroupsByActivity(
      [['designer', 'dev', 'pm']],
      [pair('dev', 'pm', at(30_000), 1)],
      NOW
    )
    expect(ordered[0].activity).toBeNull()
  })
})
