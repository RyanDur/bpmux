import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'

let historyResponse: unknown[] = []
const httpGet = vi.fn((..._args: unknown[]) => ({
  value: Promise.resolve({ kind: 'success', value: historyResponse })
}))

vi.mock('../api/http', () => ({
  http: { get: (...args: unknown[]) => httpGet(...args) },
  describeHttpError: (e: unknown) => String(e)
}))

import {
  startSession,
  getSessionSnapshot,
  subscribeSession,
  subscribeLiveMessages,
  resetSessionForTest
} from './session'

class MockWebSocket {
  static instances: MockWebSocket[] = []
  static OPEN = 1
  static CONNECTING = 0
  static CLOSING = 2
  static CLOSED = 3
  url: string
  readyState = 0
  onopen: (() => void) | null = null
  onmessage: ((ev: { data: string }) => void) | null = null
  onerror: ((ev: unknown) => void) | null = null
  onclose: ((ev: { code: number; reason: string }) => void) | null = null
  sent: string[] = []
  closed = false
  constructor(url: string) {
    this.url = url
    MockWebSocket.instances.push(this)
  }
  send(data: string): void {
    this.sent.push(data)
  }
  close(): void {
    this.closed = true
    this.readyState = MockWebSocket.CLOSED
  }
  open(): void {
    this.readyState = MockWebSocket.OPEN
    this.onopen?.()
  }
  receive(data: string): void {
    this.onmessage?.({ data })
  }
  drop(code = 1006, reason = 'lost'): void {
    this.readyState = MockWebSocket.CLOSED
    this.onclose?.({ code, reason })
  }
  static latest(): MockWebSocket {
    return MockWebSocket.instances[MockWebSocket.instances.length - 1]
  }
}

const serverMessage = (over: Record<string, unknown> = {}): string =>
  JSON.stringify({
    type: 'message',
    from: 'pm',
    text: 'hello',
    ts: '2026-05-24T00:00:00.000Z',
    ...over
  })

beforeEach(() => {
  vi.stubGlobal('WebSocket', MockWebSocket)
  MockWebSocket.instances = []
  historyResponse = []
  httpGet.mockClear()
  resetSessionForTest()
})

afterEach(() => {
  resetSessionForTest()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

describe('#1537 session — connection lifecycle as a module singleton', () => {
  test('startSession opens a socket, surfaces connecting then connected, and identifies on open', () => {
    startSession({ url: 'ws://test' })
    expect(getSessionSnapshot().status).toBe('connecting')
    expect(MockWebSocket.instances.length).toBe(1)

    MockWebSocket.latest().open()
    expect(getSessionSnapshot().status).toBe('connected')
    const identify = MockWebSocket.latest().sent.map((s) => JSON.parse(s))
    expect(identify[0]).toMatchObject({ type: 'identify', name: 'ryan', tool: 'electron' })
  })

  test('startSession is idempotent — a second call with the same url does NOT open a second socket (StrictMode-once)', () => {
    startSession({ url: 'ws://test' })
    startSession({ url: 'ws://test' })
    expect(MockWebSocket.instances.length).toBe(1)
  })

  test('getSessionSnapshot returns a STABLE reference until state actually changes (useSyncExternalStore safety)', () => {
    startSession({ url: 'ws://test' })
    const a = getSessionSnapshot()
    const b = getSessionSnapshot()
    expect(a).toBe(b)
    MockWebSocket.latest().open()
    const c = getSessionSnapshot()
    expect(c).not.toBe(a)
    expect(getSessionSnapshot()).toBe(c)
  })

  test('subscribers are notified on state change and stop after unsubscribe', () => {
    startSession({ url: 'ws://test' })
    let hits = 0
    const unsub = subscribeSession(() => {
      hits += 1
    })
    MockWebSocket.latest().open()
    expect(hits).toBeGreaterThan(0)
    const afterOpen = hits
    unsub()
    MockWebSocket.latest().receive(serverMessage({ ts: '2026-05-24T00:00:01.000Z' }))
    expect(hits).toBe(afterOpen)
  })
})

describe('#1537 session — message dedup + live-append subscription', () => {
  test('a chat message lands in messages; an identical duplicate (same ts|from|text) is deduped', () => {
    startSession({ url: 'ws://test' })
    MockWebSocket.latest().open()
    MockWebSocket.latest().receive(serverMessage())
    MockWebSocket.latest().receive(serverMessage())
    expect(getSessionSnapshot().messages.length).toBe(1)
  })

  test('subscribeLiveMessages fires once per newly-appended live message, not for dedup-dropped ones', () => {
    startSession({ url: 'ws://test' })
    MockWebSocket.latest().open()
    const seen: string[] = []
    subscribeLiveMessages((m) => seen.push(m.text))
    MockWebSocket.latest().receive(serverMessage({ text: 'one', ts: '2026-05-24T00:00:01.000Z' }))
    MockWebSocket.latest().receive(serverMessage({ text: 'one', ts: '2026-05-24T00:00:01.000Z' }))
    MockWebSocket.latest().receive(serverMessage({ text: 'two', ts: '2026-05-24T00:00:02.000Z' }))
    expect(seen).toEqual(['one', 'two'])
  })
})

describe('#1537 session — history load-once + merge (no live-notify on history)', () => {
  test('history is fetched once at init and merged into messages; a duplicate start does NOT refetch', async () => {
    historyResponse = [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }]
    startSession({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    startSession({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    const startCalls = httpGet.mock.calls.length
    startSession({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    expect(httpGet.mock.calls.length).toBe(startCalls)
  })

  test('history-merge emits ZERO live-message append-events (only live frames notify)', async () => {
    historyResponse = [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }]
    const seen: string[] = []
    subscribeLiveMessages((m) => seen.push(m.text))
    startSession({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    expect(seen).toEqual([])
  })
})

describe('#1537 session — reconnect with backoff', () => {
  test('on close, status goes disconnected and a reconnect is scheduled with growing backoff', () => {
    vi.useFakeTimers()
    startSession({ url: 'ws://test' })
    MockWebSocket.latest().open()
    expect(getSessionSnapshot().status).toBe('connected')

    MockWebSocket.latest().drop()
    expect(getSessionSnapshot().status).toBe('disconnected')
    expect(MockWebSocket.instances.length).toBe(1)

    vi.advanceTimersByTime(1000)
    expect(MockWebSocket.instances.length).toBe(2)
  })

  test('reconnectTick increments on each disconnect→connect transition', () => {
    vi.useFakeTimers()
    startSession({ url: 'ws://test' })
    MockWebSocket.latest().open()
    expect(getSessionSnapshot().reconnectTick).toBe(0)

    MockWebSocket.latest().drop()
    vi.advanceTimersByTime(1000)
    MockWebSocket.latest().open()
    expect(getSessionSnapshot().reconnectTick).toBe(1)
  })
})

describe('#1319 session — reconnect re-fetches history to recover the disconnect gap', () => {
  test('on reconnect, history is re-fetched so a group/broadcast message sent during the gap appears after recovery', async () => {
    vi.useFakeTimers()
    historyResponse = [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }]
    startSession({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    MockWebSocket.latest().open()
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )
    const callsBeforeDrop = httpGet.mock.calls.length

    MockWebSocket.latest().drop()
    expect(getSessionSnapshot().status).toBe('disconnected')

    historyResponse = [
      { from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' },
      { from: 'designer', text: 'sent-during-gap', ts: '2026-05-28T00:01:00.000Z' }
    ]

    vi.advanceTimersByTime(1000)
    expect(MockWebSocket.instances.length).toBe(2)
    MockWebSocket.latest().open()

    await vi.waitFor(() => expect(httpGet.mock.calls.length).toBeGreaterThan(callsBeforeDrop))
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'sent-during-gap')).toBe(true)
    )
  })

  test('on reconnect-refetch, a duplicate pre-gap message (already in state) does not appear twice', async () => {
    vi.useFakeTimers()
    historyResponse = [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }]
    startSession({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    MockWebSocket.latest().open()
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )

    MockWebSocket.latest().drop()
    vi.advanceTimersByTime(1000)
    MockWebSocket.latest().open()

    await vi.waitFor(() => expect(httpGet.mock.calls.length).toBeGreaterThan(2))

    const preGapCount = getSessionSnapshot().messages.filter((m) => m.text === 'pre-gap').length
    expect(preGapCount).toBe(1)
  })
})

describe('#1618 session — board-refresh migrates from the text-marker to the board_invalidated state-delta', () => {
  test('a board_invalidated state-delta bumps boardInvalidateTick (the structured board-refresh trigger)', () => {
    startSession({ url: 'ws://test' })
    MockWebSocket.latest().open()
    expect(getSessionSnapshot().boardInvalidateTick).toBe(0)
    MockWebSocket.latest().receive(
      serverMessage({
        from: 'bpmux',
        text: '',
        message_type: 'state-delta',
        state_delta: { domain: 'board_invalidated', reason: 'board changed' }
      })
    )
    expect(getSessionSnapshot().boardInvalidateTick).toBe(1)
  })

  test('a board_invalidated state-delta leaves the mirror reactions/ci/roster untouched', () => {
    startSession({ url: 'ws://test' })
    MockWebSocket.latest().open()
    const before = getSessionSnapshot().mirror
    MockWebSocket.latest().receive(
      serverMessage({
        from: 'bpmux',
        text: '',
        message_type: 'state-delta',
        state_delta: { domain: 'board_invalidated', reason: 'board changed' }
      })
    )
    expect(getSessionSnapshot().mirror).toBe(before)
  })
})
