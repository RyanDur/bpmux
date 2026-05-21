import { describe, expect, test } from 'vitest'
import { selectReactions, type AggregatedReaction } from './useReactions'
import type { ReactionAggregateWire } from '../transport/types'

describe('selectReactions — slice 5c reactions mirror-priority archetype', () => {
  test('returns mirror entry when present (mirror-priority over REST)', () => {
    const mirror: ReactionAggregateWire[] = [
      { message_key: 'k1', emoji: '👍', count: 2, users: ['ryan', 'dev'] }
    ]
    const rest: AggregatedReaction[] = [
      { message_key: 'k1', emoji: '👍', count: 1, users: ['ryan'] }
    ]
    expect(selectReactions(mirror, rest)).toBe(mirror)
  })

  test('falls back to REST aggregated when mirror entry missing for channel', () => {
    const rest: AggregatedReaction[] = [
      { message_key: 'k2', emoji: '🎉', count: 1, users: ['ryan'] }
    ]
    expect(selectReactions(undefined, rest)).toBe(rest)
  })

  test('returns mirror entry even when empty (mirror is authoritative for "no reactions")', () => {
    const mirror: ReactionAggregateWire[] = []
    const rest: AggregatedReaction[] = [{ message_key: 'k', emoji: '👍', count: 1, users: ['dev'] }]
    expect(selectReactions(mirror, rest)).toBe(mirror)
  })
})
