import { describe, expect, it } from 'vitest'
import { emptySnapshot, wsReducer, type WsSnapshot } from './wsReducer'
import {
  chatSendRequested,
  connected,
  connecting,
  disconnected,
  historyLoaded,
  messageReceived,
  reconnected,
  syncStatusBumped
} from './wsActions'
import type { ChatMessage } from './types'

const chat = (over: Partial<ChatMessage> & { ts: string; from: string; text: string }): ChatMessage => ({
  id: `${over.ts}-x`,
  to: null,
  addressee: null,
  messageType: 'chat',
  ...over
})

describe('wsReducer — connection status events', () => {
  it('connecting is a no-op (same reference) when already connecting', () => {
    const s = emptySnapshot('ws://x')
    expect(wsReducer(s, connecting())).toBe(s)
  })

  it('connected sets status; reconnected also bumps reconnectTick', () => {
    const s = emptySnapshot('ws://x')
    expect(wsReducer(s, connected()).status).toBe('connected')
    expect(wsReducer(s, reconnected())).toMatchObject({ status: 'connected', reconnectTick: 1 })
    expect(wsReducer(s, disconnected()).status).toBe('disconnected')
  })
})

describe('wsReducer — message dedup', () => {
  it('appends a new message and dedups an identical one (same ref on dup)', () => {
    const s0 = emptySnapshot('ws://x')
    const m = chat({ ts: '2026-01-01T00:00:00Z', from: 'pm', text: 'hi' })
    const s1 = wsReducer(s0, messageReceived(m))
    expect(s1.messages).toHaveLength(1)
    const s2 = wsReducer(s1, messageReceived(chat({ ts: m.ts, from: 'pm', text: 'hi' })))
    expect(s2).toBe(s1)
  })
})

describe('wsReducer — history merge', () => {
  it('merges history before existing messages, deduping by ts|from|text', () => {
    const live = chat({ ts: '2026-01-01T00:00:02Z', from: 'pm', text: 'live' })
    const s1 = wsReducer(emptySnapshot('ws://x'), messageReceived(live))
    const s2 = wsReducer(
      s1,
      historyLoaded([
        { from: 'designer', text: 'old', ts: '2026-01-01T00:00:01Z' },
        { from: 'pm', text: 'live', ts: '2026-01-01T00:00:02Z' }
      ])
    )
    expect(s2.messages.map((m) => m.text)).toEqual(['old', 'live'])
  })
})

describe('wsReducer — ticks and command pass-through', () => {
  it('syncStatusBumped increments syncStatusTick', () => {
    const s = emptySnapshot('ws://x')
    expect(wsReducer(s, syncStatusBumped()).syncStatusTick).toBe(1)
  })

  it('a command action (no state meaning) returns the same state reference', () => {
    const s: WsSnapshot = emptySnapshot('ws://x')
    expect(wsReducer(s, chatSendRequested('hi'))).toBe(s)
  })
})
