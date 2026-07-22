import { describe, expect, it } from 'vitest'
import { emptySnapshot, stateReducer, type StateSnapshot } from './reducer'
import {
  chatSendRequested,
  connected,
  connecting,
  disconnected,
  historyLoaded,
  messageReceived,
  reconnected
} from './actions'
import type { ChatMessage } from '../transport/types'

const chat = (
  over: Partial<ChatMessage> & { ts: string; from: string; text: string }
): ChatMessage => ({
  id: `${over.ts}-x`,
  to: null,
  addressee: null,
  messageType: 'chat',
  ...over
})

describe('stateReducer — connection status events', () => {
  it('connecting is a no-op (same reference) when already connecting', () => {
    const s = emptySnapshot('ws://x')
    expect(stateReducer(s, connecting())).toBe(s)
  })

  it('connected sets status; reconnected also bumps reconnectTick', () => {
    const s = emptySnapshot('ws://x')
    expect(stateReducer(s, connected()).status).toBe('connected')
    expect(stateReducer(s, reconnected())).toMatchObject({
      status: 'connected',
      reconnectTick: 1
    })
    expect(stateReducer(s, disconnected()).status).toBe('disconnected')
  })
})

describe('stateReducer — message dedup', () => {
  it('appends a new message and dedups an identical one (same ref on dup)', () => {
    const s0 = emptySnapshot('ws://x')
    const m = chat({ ts: '2026-01-01T00:00:00Z', from: 'pm', text: 'hi' })
    const s1 = stateReducer(s0, messageReceived(m))
    expect(s1.messages).toHaveLength(1)
    const s2 = stateReducer(s1, messageReceived(chat({ ts: m.ts, from: 'pm', text: 'hi' })))
    expect(s2).toBe(s1)
  })
})

describe('stateReducer — history merge', () => {
  it('merges history before existing messages, deduping by ts|from|text', () => {
    const live = chat({ ts: '2026-01-01T00:00:02Z', from: 'pm', text: 'live' })
    const s1 = stateReducer(emptySnapshot('ws://x'), messageReceived(live))
    const s2 = stateReducer(
      s1,
      historyLoaded([
        { from: 'designer', text: 'old', ts: '2026-01-01T00:00:01Z' },
        { from: 'pm', text: 'live', ts: '2026-01-01T00:00:02Z' }
      ])
    )
    expect(s2.messages.map((m) => m.text)).toEqual(['old', 'live'])
  })
})

describe('stateReducer — ticks and command pass-through', () => {
  it('a command action (no state meaning) returns the same state reference', () => {
    const s: StateSnapshot = emptySnapshot('ws://x')
    expect(stateReducer(s, chatSendRequested('hi'))).toBe(s)
  })
})
