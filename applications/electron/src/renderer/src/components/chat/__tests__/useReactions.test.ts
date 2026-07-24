import { afterEach, describe, expect, test } from 'vitest'
import { cleanup, renderHook } from '@testing-library/react'
import { useReactions } from '../useReactions'
import { dispatchState, resetStateForTest } from '@state/store'
import { mirrorDeltaReceived } from '@state/actions'
import { captureSentFramesForTest } from '@state'
import type { ReactionAggregateWire } from '@transport/types'

const pushReactions = (channel: string, aggregated: ReactionAggregateWire[]): void =>
  dispatchState(
    mirrorDeltaReceived({ domain: 'reactions', channel, aggregated }, '2026-01-01T00:00:00Z')
  )

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('useReactions — channel-scoped aggregation from the session store', () => {
  test('groups the channel’s reactions by message_key', () => {
    pushReactions('broadcast', [
      { message_key: 'm1', emoji: '👍', count: 2, users: ['owner', 'pm'] },
      { message_key: 'm1', emoji: '🎉', count: 1, users: ['designer'] },
      { message_key: 'm2', emoji: '🚀', count: 1, users: ['dev'] }
    ])
    const { result } = renderHook(() => useReactions({ channel: 'broadcast' }))
    expect(result.current.byMessage.get('m1')?.map((r) => r.emoji)).toEqual(['👍', '🎉'])
    expect(result.current.byMessage.get('m2')?.map((r) => r.emoji)).toEqual(['🚀'])
  })

  test('a different channel sees none of another channel’s reactions', () => {
    pushReactions('broadcast', [{ message_key: 'm1', emoji: '👍', count: 1, users: ['owner'] }])
    const { result } = renderHook(() => useReactions({ channel: 'dev+pm' }))
    expect(result.current.byMessage.size).toBe(0)
  })
})

describe('useReactions — toggle sends a reaction_toggle over the WS (no REST)', () => {
  test('toggle dispatches a reaction_toggle frame carrying channel/message_key/emoji/user', () => {
    const frames = captureSentFramesForTest()
    const { result } = renderHook(() => useReactions({ channel: 'broadcast' }))
    result.current.toggle('m1', '👍', 'dev')
    const toggleFrame = frames.find((f) => f.type === 'reaction_toggle')
    expect(toggleFrame).toEqual({
      type: 'reaction_toggle',
      channel: 'broadcast',
      message_key: 'm1',
      emoji: '👍',
      user: 'dev'
    })
  })
})
