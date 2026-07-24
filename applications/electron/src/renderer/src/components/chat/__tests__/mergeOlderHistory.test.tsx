import { describe, expect, test } from 'vitest'
import { mergeOlderHistory } from '../useMessageHistory'
import type { ChatMessage } from '@transport/types'

const mkLive = (i: number): ChatMessage => ({
  id: `live-${i}`,
  from: 'user',
  text: `live ${i}`,
  ts: `2026-05-03T03:${String(50 + i).padStart(2, '0')}:00.000Z`,
  to: null,
  addressee: null,
  messageType: null
})

const mkOlderRow = (
  i: number
): {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
} => ({
  from: 'user',
  text: `older ${i}`,
  ts: `2026-05-03T03:${String(10 + i).padStart(2, '0')}:00.000Z`
})

describe('mergeOlderHistory — pure dedupe + chronological merge', () => {
  test('empty inputs return empty prepended buffer', () => {
    expect(mergeOlderHistory([], [], [])).toEqual([])
  })

  test('prepends fetched rows in chronological order', () => {
    const older = [mkOlderRow(2), mkOlderRow(0), mkOlderRow(1)]
    const result = mergeOlderHistory([], older, [])
    expect(result.map((m) => m.text)).toEqual(['older 0', 'older 1', 'older 2'])
  })

  test('dedupes against live messages by ts+from+text', () => {
    const live = [mkLive(0), mkLive(1)]
    const older = [{ from: live[0].from, text: live[0].text, ts: live[0].ts }, mkOlderRow(0)]
    const result = mergeOlderHistory([], older, live)
    expect(result.length).toBe(1)
    expect(result[0].text).toBe('older 0')
  })

  test('dedupes against existing prepended buffer (avoids double-prepend on retry)', () => {
    const live = [mkLive(0)]
    const existing: ChatMessage[] = [
      {
        id: 'hist-older-existing',
        from: 'user',
        text: 'older 0',
        ts: '2026-05-03T03:10:00.000Z',
        to: null,
        addressee: null,
        messageType: null
      }
    ]
    const older = [mkOlderRow(0)]
    const result = mergeOlderHistory(existing, older, live)
    expect(result.length).toBe(1)
    expect(result[0].text).toBe('older 0')
  })

  test('prepends new older rows BEFORE existing prepended buffer (chronological)', () => {
    const live = [mkLive(0)]
    const existing: ChatMessage[] = [
      {
        id: 'hist-existing',
        from: 'user',
        text: 'older 5',
        ts: '2026-05-03T03:15:00.000Z',
        to: null,
        addressee: null,
        messageType: null
      }
    ]
    const older = [mkOlderRow(0), mkOlderRow(1)]
    const result = mergeOlderHistory(existing, older, live)
    expect(result.map((m) => m.text)).toEqual(['older 0', 'older 1', 'older 5'])
  })

  test('preserves stable id format for prepended rows', () => {
    const older = [mkOlderRow(0)]
    const result = mergeOlderHistory([], older, [])
    expect(result[0].id).toMatch(/^hist-older-/)
  })
})
