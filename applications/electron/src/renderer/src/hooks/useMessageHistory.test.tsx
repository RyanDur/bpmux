import { describe, expect, test } from 'vitest'
import { mergeOlderHistory } from './useMessageHistory'
import type { ChatMessage } from '../transport/types'

// #1153 slice 3: useMessageHistory hook. The hook's React-state +
// fetch machinery is exercised in slice 4's integration test (Chat
// component drives loadOlder via scroll-up-near-top). Slice 3 unit
// tests cover the pure merge-and-dedupe helper that loads-older
// rows go through — the load-bearing logic where boundary-overlap +
// race-conditions could double-render messages.

const mkLive = (i: number): ChatMessage => ({
  id: `live-${i}`,
  from: 'user',
  text: `live ${i}`,
  ts: `2026-05-03T03:${String(50 + i).padStart(2, '0')}:00.000Z`,
  to: null,
  addressee: null,
  messageType: null
})

const mkOlderRow = (i: number): {
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

describe('#1153 mergeOlderHistory — pure dedupe + chronological merge', () => {
  test('empty inputs return empty prepended buffer', () => {
    expect(mergeOlderHistory([], [], [])).toEqual([])
  })

  test('prepends fetched rows in chronological order', () => {
    // Server returns newest-first; helper sorts chronological.
    const older = [mkOlderRow(2), mkOlderRow(0), mkOlderRow(1)]
    const result = mergeOlderHistory([], older, [])
    expect(result.map((m) => m.text)).toEqual(['older 0', 'older 1', 'older 2'])
  })

  test('dedupes against live messages by ts+from+text', () => {
    const live = [mkLive(0), mkLive(1)]
    // Server includes a row that's already in live (boundary-overlap
    // race) plus a genuine older row.
    const older = [
      { from: live[0].from, text: live[0].text, ts: live[0].ts },
      mkOlderRow(0)
    ]
    const result = mergeOlderHistory([], older, live)
    // Only the genuine older row makes it through.
    expect(result.length).toBe(1)
    expect(result[0].text).toBe('older 0')
  })

  test('dedupes against existing prepended buffer (avoids double-prepend on retry)', () => {
    const live = [mkLive(0)]
    // First fetch already added "older 0" to prependedHistory.
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
    // Second fetch returns the same row (e.g., user scrolled up,
    // scrolled down, scrolled up again with same anchor).
    const older = [mkOlderRow(0)]
    const result = mergeOlderHistory(existing, older, live)
    // No duplicate; existing buffer unchanged.
    expect(result.length).toBe(1)
    expect(result[0].text).toBe('older 0')
  })

  test('prepends new older rows BEFORE existing prepended buffer (chronological)', () => {
    const live = [mkLive(0)]
    // First fetch added "older 5" earlier.
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
    // Second fetch returns even-older rows.
    const older = [mkOlderRow(0), mkOlderRow(1)]
    const result = mergeOlderHistory(existing, older, live)
    expect(result.map((m) => m.text)).toEqual(['older 0', 'older 1', 'older 5'])
  })

  test('preserves stable id format for prepended rows', () => {
    const older = [mkOlderRow(0)]
    const result = mergeOlderHistory([], older, [])
    // Format is hist-older-<ts>-<idx> so MessageList's React-key path
    // stays stable across re-renders.
    expect(result[0].id).toMatch(/^hist-older-/)
  })
})
