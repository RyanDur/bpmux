import { describe, expect, test } from 'vitest'
import { shouldRefetchMarkers } from './useIterationMarkers'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'

const message = (text: string, ts = '2026-05-10T04:00:00Z'): ChatMessage => ({
  id: `msg-${ts}`,
  from: 'bpmux',
  text,
  ts,
  to: null,
  addressee: null,
  messageType: null
})

describe('#1255 shouldRefetchMarkers — gate-logic for hot-update on board-invalidate', () => {
  test('undefined messages → false (parent didnt provide invalidation signal)', () => {
    expect(shouldRefetchMarkers(undefined, 0)).toBe(false)
  })

  test('empty messages → false (no signal)', () => {
    expect(shouldRefetchMarkers([], 0)).toBe(false)
  })

  test('messages.length <= prevSeenCount → false (no new messages since last check)', () => {
    const msgs: ChatMessage[] = [message(BOARD_INVALIDATE_MARKER)]
    expect(shouldRefetchMarkers(msgs, 1)).toBe(false)
    expect(shouldRefetchMarkers(msgs, 5)).toBe(false)
  })

  test('new non-invalidate message → false (refetch only on invalidate)', () => {
    const msgs: ChatMessage[] = [message('a regular chat message')]
    expect(shouldRefetchMarkers(msgs, 0)).toBe(false)
  })

  test('new BOARD_INVALIDATE_MARKER message → true (refetch fires)', () => {
    const msgs: ChatMessage[] = [message(BOARD_INVALIDATE_MARKER)]
    expect(shouldRefetchMarkers(msgs, 0)).toBe(true)
  })

  test('mix of new messages with one invalidate → true', () => {
    const msgs: ChatMessage[] = [
      message('regular 1', '2026-05-10T04:00:00Z'),
      message(BOARD_INVALIDATE_MARKER, '2026-05-10T04:01:00Z'),
      message('regular 2', '2026-05-10T04:02:00Z')
    ]
    expect(shouldRefetchMarkers(msgs, 0)).toBe(true)
  })

  test('pre-existing invalidate ABOVE prevSeenCount cutoff → false (boundary discipline)', () => {
    // 2 messages were "seen" already; only the third (regular) is new
    // since seenCount=2. The invalidate at index 0 is below the cutoff
    // and must NOT trigger refetch.
    const msgs: ChatMessage[] = [
      message(BOARD_INVALIDATE_MARKER, '2026-05-10T03:59:00Z'),
      message('seen regular', '2026-05-10T04:00:00Z'),
      message('new regular', '2026-05-10T04:01:00Z')
    ]
    expect(shouldRefetchMarkers(msgs, 2)).toBe(false)
  })

  test('new invalidate AT prevSeenCount boundary → true', () => {
    // seenCount=1; index 1 is the first new message; it carries the
    // invalidate.
    const msgs: ChatMessage[] = [
      message('seen regular', '2026-05-10T04:00:00Z'),
      message(BOARD_INVALIDATE_MARKER, '2026-05-10T04:01:00Z')
    ]
    expect(shouldRefetchMarkers(msgs, 1)).toBe(true)
  })
})
