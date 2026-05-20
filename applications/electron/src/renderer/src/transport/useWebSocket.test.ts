import { describe, expect, test } from 'vitest'
import { CAP_TRIGGER_THRESHOLD, capMessages, capPerPush, MAX_MESSAGES } from './useWebSocket'
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

describe('capPerPush — UX guard + threshold', () => {
  const buffer = (n: number): ChatMessage[] => Array.from({ length: n }, (_, i) => mkMsg(i))

  test('passes through unchanged when below trigger threshold', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD)
    expect(capPerPush(msgs, true)).toBe(msgs)
  })

  test('passes through unchanged when at-bottom is false (preserve scroll)', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD + 100)
    expect(capPerPush(msgs, false)).toBe(msgs)
  })

  test('caps to MAX_MESSAGES when at-bottom AND above trigger threshold', () => {
    const msgs = buffer(CAP_TRIGGER_THRESHOLD + 1)
    const capped = capPerPush(msgs, true)
    expect(capped.length).toBe(MAX_MESSAGES)
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
