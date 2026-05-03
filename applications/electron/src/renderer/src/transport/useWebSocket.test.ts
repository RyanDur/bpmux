import { describe, expect, test } from 'vitest'
import {
  CAP_TRIGGER_THRESHOLD,
  capMessages,
  capMessagesPreservingPinned,
  capPerPush,
  MAX_MESSAGES
} from './useWebSocket'
import type { ChatMessage } from './types'

const mkMsg = (i: number): ChatMessage => ({
  id: `id-${i}`,
  from: 'user',
  text: `msg ${i}`,
  ts: `2026-05-03T03:${String(i).padStart(2, '0')}:00.000Z`,
  to: null,
  addressee: null,
  messageType: null
})

const pinKey = (m: ChatMessage): string => `${m.ts}|${m.from}`

describe('capMessages — bounded slice', () => {
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
    expect(capped.length).toBe(3)
    expect(capped.map((m) => m.text)).toEqual(['msg 2', 'msg 3', 'msg 4'])
  })

  test('preserves chronological order across cap', () => {
    const msgs = Array.from({ length: 1000 }, (_, i) => mkMsg(i))
    const capped = capMessages(msgs, 500)
    expect(capped.length).toBe(500)
    expect(capped[0].text).toBe('msg 500')
    expect(capped[499].text).toBe('msg 999')
  })
})

describe('capMessagesPreservingPinned — pinned exempt from drop-tail', () => {
  test('passes through when below cap (no slice needed)', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2)]
    expect(capMessagesPreservingPinned(msgs, 10, new Set())).toBe(msgs)
  })

  test('drops un-pinned older messages when above cap', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2), mkMsg(3), mkMsg(4)]
    const capped = capMessagesPreservingPinned(msgs, 3, new Set())
    expect(capped.map((m) => m.text)).toEqual(['msg 2', 'msg 3', 'msg 4'])
  })

  test('preserves pinned messages outside the most-recent window', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2), mkMsg(3), mkMsg(4)]
    const pinned = new Set([pinKey(msgs[0])])
    const capped = capMessagesPreservingPinned(msgs, 3, pinned)
    expect(capped.map((m) => m.text)).toEqual(['msg 0', 'msg 2', 'msg 3', 'msg 4'])
  })

  test('preserves multiple pinned messages in chronological order', () => {
    const msgs = Array.from({ length: 10 }, (_, i) => mkMsg(i))
    const pinned = new Set([pinKey(msgs[1]), pinKey(msgs[3]), pinKey(msgs[5])])
    const capped = capMessagesPreservingPinned(msgs, 3, pinned)
    expect(capped.map((m) => m.text)).toEqual([
      'msg 1',
      'msg 3',
      'msg 5',
      'msg 7',
      'msg 8',
      'msg 9'
    ])
  })

  test('does not duplicate pinned messages already in the recent window', () => {
    const msgs = [mkMsg(0), mkMsg(1), mkMsg(2), mkMsg(3), mkMsg(4)]
    const pinned = new Set([pinKey(msgs[4])])
    const capped = capMessagesPreservingPinned(msgs, 3, pinned)
    expect(capped.length).toBe(3)
    expect(capped.map((m) => m.text)).toEqual(['msg 2', 'msg 3', 'msg 4'])
  })
})

describe('capPerPush — UX guard + threshold + pinned exemption', () => {
  const buffer = (n: number): ChatMessage[] =>
    Array.from({ length: n }, (_, i) => mkMsg(i))

  test('passes through unchanged when below trigger threshold', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD)
    expect(capPerPush(msgs, true, new Set())).toBe(msgs)
  })

  test('passes through unchanged when at-bottom is false (preserve scroll)', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD + 100)
    // User reading older history → cap suspended even though length
    // exceeds threshold. Critical for "no-jank-on-scroll-up" AC.
    expect(capPerPush(msgs, false, new Set())).toBe(msgs)
  })

  test('caps to MAX_MESSAGES when at-bottom AND above trigger threshold', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD + 1)
    const capped = capPerPush(msgs, true, new Set())
    expect(capped.length).toBe(MAX_MESSAGES)
  })

  test('preserves pinned older messages when capping', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD + 1)
    const pinned = new Set([pinKey(msgs[0])])
    const capped = capPerPush(msgs, true, pinned)
    expect(capped[0].text).toBe('msg 0')
    expect(capped.length).toBe(MAX_MESSAGES + 1)
  })
})

describe('exports', () => {
  test('MAX_MESSAGES is 500', () => {
    expect(MAX_MESSAGES).toBe(500)
  })

  test('CAP_TRIGGER_THRESHOLD is 2L per AC', () => {
    expect(CAP_TRIGGER_THRESHOLD).toBe(MAX_MESSAGES * 2)
  })
})
