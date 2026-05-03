import { describe, expect, test } from 'vitest'
import { capMessages, MAX_MESSAGES } from './useWebSocket'
import type { ChatMessage } from './types'

// #1153: cap-helper unit tests. The hook itself isn't covered without
// renderHook infrastructure (none in this codebase yet); the cap logic
// is the load-bearing piece for the bounded-array contract, so we
// extract + test it pure.

const mkMsg = (i: number): ChatMessage => ({
  id: `id-${i}`,
  from: 'user',
  text: `msg ${i}`,
  ts: `2026-05-03T03:${String(i).padStart(2, '0')}:00.000Z`,
  to: null,
  addressee: null,
  messageType: null
})

describe('#1153 capMessages — bounded in-memory array', () => {
  test('returns input unchanged when below cap', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2)]
    expect(capMessages(msgs, 10)).toBe(msgs)
  })

  test('returns input unchanged when exactly at cap', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2)]
    expect(capMessages(msgs, 3)).toBe(msgs)
  })

  test('slices to most-recent N when above cap', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2), mkMsg(3), mkMsg(4)]
    const capped = capMessages(msgs, 3)
    // Drop-tail: oldest first dropped; most-recent N kept in same
    // chronological order.
    expect(capped.length).toBe(3)
    expect(capped.map((m) => m.text)).toEqual(['msg 2', 'msg 3', 'msg 4'])
  })

  test('preserves chronological order across cap', () => {
    // Long-session simulation: 1000 messages pushed, cap at 500.
    // Verify the cap takes the most-recent 500 in original order.
    const msgs = Array.from({ length: 1000 }, (_, i) => mkMsg(i))
    const capped = capMessages(msgs, 500)
    expect(capped.length).toBe(500)
    expect(capped[0].text).toBe('msg 500')
    expect(capped[499].text).toBe('msg 999')
  })

  test('MAX_MESSAGES default exposed for downstream callers', () => {
    // MAX_MESSAGES is the default cap used in the hook; downstream
    // tests / callers that need to verify the bounded-array contract
    // can import this constant directly.
    expect(MAX_MESSAGES).toBe(500)
  })
})
