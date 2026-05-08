import { describe, expect, test } from 'vitest'
import { selectPins, wireToPin, type Pin } from './usePins'
import type { PinnedMessageWire } from '../transport/types'

const wire = (overrides: Partial<PinnedMessageWire> = {}): PinnedMessageWire => ({
  message_key: 'k1',
  pinner: 'ryan',
  ts: '2026-05-08T00:00:00Z',
  text: 'pinned text',
  from: 'ryan',
  pinned_at: '2026-05-08T00:00:01Z',
  ...overrides
})

describe('selectPins — slice 5f pins mirror-priority archetype', () => {
  test('returns mapped mirror entry when present (mirror-priority over REST)', () => {
    const mirror: PinnedMessageWire[] = [wire({ message_key: 'm1', pinner: 'ryan' })]
    const rest: Pin[] = [
      {
        message_key: 'm1',
        from: 'ryan',
        ts: 't',
        text: 'stale',
        pinned_by: 'ryan',
        pinned_at: 't'
      }
    ]
    const result = selectPins(mirror, rest)
    expect(result.length).toBe(1)
    expect(result[0].text).toBe('pinned text')
    expect(result[0].pinned_by).toBe('ryan')
  })

  test('falls back to REST pins when mirror entry missing for channel', () => {
    const rest: Pin[] = [
      {
        message_key: 'm2',
        from: 'dev',
        ts: 't',
        text: 'rest only',
        pinned_by: 'dev',
        pinned_at: 't'
      }
    ]
    expect(selectPins(undefined, rest)).toBe(rest)
  })

  test('returns empty mapped array when mirror entry is empty (mirror is authoritative for "no pins")', () => {
    const rest: Pin[] = [
      {
        message_key: 'm',
        from: 'dev',
        ts: 't',
        text: 'should not show',
        pinned_by: 'dev',
        pinned_at: 't'
      }
    ]
    const result = selectPins([], rest)
    expect(result.length).toBe(0)
  })
})

describe('wireToPin — slice 5f vocabulary translation (pinner ↔ pinned_by)', () => {
  test('renames pinner → pinned_by while preserving other fields', () => {
    const pin = wireToPin(wire({ pinner: 'designer', from: 'dev' }))
    expect(pin.pinned_by).toBe('designer')
    expect(pin.from).toBe('dev')
    expect(pin.text).toBe('pinned text')
    expect(pin.pinned_at).toBe('2026-05-08T00:00:01Z')
  })
})
