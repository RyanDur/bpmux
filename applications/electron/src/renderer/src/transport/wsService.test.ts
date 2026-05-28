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
  ensureWsStarted,
  getWsSnapshot,
  subscribeWs,
  subscribeWsLiveMessages,
  __resetWsForTest
} from './wsService'

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
  __resetWsForTest()
})

afterEach(() => {
  __resetWsForTest()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

describe('#1537 wsService — connection lifecycle as a module singleton', () => {
  test('ensureWsStarted opens a socket, surfaces connecting then connected, and identifies on open', () => {
    ensureWsStarted({ url: 'ws://test' })
    expect(getWsSnapshot().status).toBe('connecting')
    expect(MockWebSocket.instances.length).toBe(1)

    MockWebSocket.latest().open()
    expect(getWsSnapshot().status).toBe('connected')
    const identify = MockWebSocket.latest().sent.map((s) => JSON.parse(s))
    expect(identify[0]).toMatchObject({ type: 'identify', name: 'ryan', tool: 'electron' })
  })

  test('ensureWsStarted is idempotent — a second call with the same url does NOT open a second socket (StrictMode-once)', () => {
    ensureWsStarted({ url: 'ws://test' })
    ensureWsStarted({ url: 'ws://test' })
    expect(MockWebSocket.instances.length).toBe(1)
  })

  test('getWsSnapshot returns a STABLE reference until state actually changes (useSyncExternalStore safety)', () => {
    ensureWsStarted({ url: 'ws://test' })
    const a = getWsSnapshot()
    const b = getWsSnapshot()
    expect(a).toBe(b)
    MockWebSocket.latest().open()
    const c = getWsSnapshot()
    expect(c).not.toBe(a)
    expect(getWsSnapshot()).toBe(c)
  })

  test('subscribers are notified on state change and stop after unsubscribe', () => {
    ensureWsStarted({ url: 'ws://test' })
    let hits = 0
    const unsub = subscribeWs(() => {
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

describe('#1537 wsService — message dedup + live-append subscription', () => {
  test('a chat message lands in messages; an identical duplicate (same ts|from|text) is deduped', () => {
    ensureWsStarted({ url: 'ws://test' })
    MockWebSocket.latest().open()
    MockWebSocket.latest().receive(serverMessage())
    MockWebSocket.latest().receive(serverMessage())
    expect(getWsSnapshot().messages.length).toBe(1)
  })

  test('subscribeWsLiveMessages fires once per newly-appended live message, not for dedup-dropped ones', () => {
    ensureWsStarted({ url: 'ws://test' })
    MockWebSocket.latest().open()
    const seen: string[] = []
    subscribeWsLiveMessages((m) => seen.push(m.text))
    MockWebSocket.latest().receive(serverMessage({ text: 'one', ts: '2026-05-24T00:00:01.000Z' }))
    MockWebSocket.latest().receive(serverMessage({ text: 'one', ts: '2026-05-24T00:00:01.000Z' }))
    MockWebSocket.latest().receive(serverMessage({ text: 'two', ts: '2026-05-24T00:00:02.000Z' }))
    expect(seen).toEqual(['one', 'two'])
  })
})

describe('#1537 wsService — history load-once + merge (no live-notify on history)', () => {
  test('history is fetched once at init and merged into messages; a duplicate start does NOT refetch', async () => {
    historyResponse = [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }]
    ensureWsStarted({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    ensureWsStarted({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() =>
      expect(getWsSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    const startCalls = httpGet.mock.calls.length
    ensureWsStarted({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    expect(httpGet.mock.calls.length).toBe(startCalls)
  })

  test('history-merge emits ZERO live-message append-events (only live frames notify)', async () => {
    historyResponse = [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }]
    const seen: string[] = []
    subscribeWsLiveMessages((m) => seen.push(m.text))
    ensureWsStarted({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() =>
      expect(getWsSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    expect(seen).toEqual([])
  })
})

describe('#1537 wsService — reconnect with backoff', () => {
  test('on close, status goes disconnected and a reconnect is scheduled with growing backoff', () => {
    vi.useFakeTimers()
    ensureWsStarted({ url: 'ws://test' })
    MockWebSocket.latest().open()
    expect(getWsSnapshot().status).toBe('connected')

    MockWebSocket.latest().drop()
    expect(getWsSnapshot().status).toBe('disconnected')
    expect(MockWebSocket.instances.length).toBe(1)

    vi.advanceTimersByTime(1000)
    expect(MockWebSocket.instances.length).toBe(2)
  })

  test('reconnectTick increments on each disconnect→connect transition', () => {
    vi.useFakeTimers()
    ensureWsStarted({ url: 'ws://test' })
    MockWebSocket.latest().open()
    expect(getWsSnapshot().reconnectTick).toBe(0)

    MockWebSocket.latest().drop()
    vi.advanceTimersByTime(1000)
    MockWebSocket.latest().open()
    expect(getWsSnapshot().reconnectTick).toBe(1)
  })
})

describe('#1319 wsService — reconnect re-fetches history to recover the disconnect gap', () => {
  test('on reconnect, history is re-fetched so a group/broadcast message sent during the gap appears after recovery', async () => {
    vi.useFakeTimers()
    historyResponse = [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }]
    ensureWsStarted({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    MockWebSocket.latest().open()
    await vi.waitFor(() =>
      expect(getWsSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )
    const callsBeforeDrop = httpGet.mock.calls.length

    MockWebSocket.latest().drop()
    expect(getWsSnapshot().status).toBe('disconnected')

    historyResponse = [
      { from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' },
      { from: 'designer', text: 'sent-during-gap', ts: '2026-05-28T00:01:00.000Z' }
    ]

    vi.advanceTimersByTime(1000)
    expect(MockWebSocket.instances.length).toBe(2)
    MockWebSocket.latest().open()

    await vi.waitFor(() =>
      expect(httpGet.mock.calls.length).toBeGreaterThan(callsBeforeDrop)
    )
    await vi.waitFor(() =>
      expect(getWsSnapshot().messages.some((m) => m.text === 'sent-during-gap')).toBe(true)
    )
  })

  test('on reconnect-refetch, a duplicate pre-gap message (already in state) does not appear twice', async () => {
    vi.useFakeTimers()
    historyResponse = [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }]
    ensureWsStarted({ url: 'ws://test', apiBase: 'http://api', authToken: 'tok' })
    MockWebSocket.latest().open()
    await vi.waitFor(() =>
      expect(getWsSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )

    MockWebSocket.latest().drop()
    vi.advanceTimersByTime(1000)
    MockWebSocket.latest().open()

    await vi.waitFor(() => expect(httpGet.mock.calls.length).toBeGreaterThan(2))

    const preGapCount = getWsSnapshot().messages.filter((m) => m.text === 'pre-gap').length
    expect(preGapCount).toBe(1)
  })
})
