import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, renderHook } from '@testing-library/react'
import { useReactions } from './useReactions'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'
import type { ReactionAggregateWire } from '../transport/types'

const pushReactions = (channel: string, aggregated: ReactionAggregateWire[]): void =>
  dispatchSession(
    mirrorDeltaReceived({ domain: 'reactions', channel, aggregated }, '2026-01-01T00:00:00Z')
  )

afterEach(() => {
  cleanup()
  resetSessionForTest()
})

describe('useReactions — channel-scoped aggregation from the session store', () => {
  test('groups the channel’s reactions by message_key', () => {
    pushReactions('broadcast', [
      { message_key: 'm1', emoji: '👍', count: 2, users: ['ryan', 'pm'] },
      { message_key: 'm1', emoji: '🎉', count: 1, users: ['designer'] },
      { message_key: 'm2', emoji: '🚀', count: 1, users: ['dev'] }
    ])
    const { result } = renderHook(() =>
      useReactions({ apiBase: undefined, authToken: undefined, channel: 'broadcast' })
    )
    expect(result.current.byMessage.get('m1')?.map((r) => r.emoji)).toEqual(['👍', '🎉'])
    expect(result.current.byMessage.get('m2')?.map((r) => r.emoji)).toEqual(['🚀'])
  })

  test('a different channel sees none of another channel’s reactions', () => {
    pushReactions('broadcast', [{ message_key: 'm1', emoji: '👍', count: 1, users: ['ryan'] }])
    const { result } = renderHook(() =>
      useReactions({ apiBase: undefined, authToken: undefined, channel: 'dev+pm' })
    )
    expect(result.current.byMessage.size).toBe(0)
  })
})
