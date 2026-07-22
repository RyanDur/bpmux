import { describe, expect, test } from 'vitest'
import { classOf } from './useMessageFilter'

describe('#1456 classOf — exhaustive remap (no bpmux-class broadcast leaks to chat)', () => {
  test('genuine chat (null / undefined / "chat") classifies as chat', () => {
    expect(classOf(null)).toBe('chat')
    expect(classOf(undefined)).toBe('chat')
    expect(classOf('chat')).toBe('chat')
  })

  test('legacy system / activity / insight remap unchanged', () => {
    expect(classOf('system')).toBe('bpmux')
    expect(classOf('activity')).toBe('events')
    expect(classOf('insight')).toBe('events')
  })

  test('state-delta + state-snapshot classify as events, not chat (leak fix)', () => {
    expect(classOf('state-delta')).toBe('events')
    expect(classOf('state-snapshot')).toBe('events')
  })

  test('nudge classifies as events, not chat (leak fix)', () => {
    expect(classOf('nudge')).toBe('events')
  })

  test('every wire message_type the bridge emits classifies non-chat unless genuinely chat', () => {
    const operationalWireTypes = [
      'system',
      'activity',
      'insight',
      'nudge',
      'state-delta',
      'state-snapshot'
    ]
    for (const wireType of operationalWireTypes) {
      expect(classOf(wireType)).not.toBe('chat')
    }
  })
})
