import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import {
  BrowserWebSocket,
  startTestWsServer,
  type TestWsConnection,
  type TestWsServer
} from '../../../../test/wsTestServer'

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

const serverMessage = (over: Record<string, unknown> = {}): string =>
  JSON.stringify({
    type: 'message',
    from: 'pm',
    text: 'hello',
    ts: '2026-05-24T00:00:00.000Z',
    ...over
  })

let server: TestWsServer

const connect = async (apiBase?: string, authToken?: string): Promise<TestWsConnection> => {
  startSession({ url: server.url, apiBase, authToken })
  const conn = await server.waitForConnection()
  await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('connected'))
  return conn
}

beforeEach(async () => {
  vi.stubGlobal('WebSocket', BrowserWebSocket)
  server = await startTestWsServer()
  historyResponse = []
  httpGet.mockClear()
  resetSessionForTest()
})

afterEach(async () => {
  resetSessionForTest()
  await server.close()
  vi.unstubAllGlobals()
})

describe('#1537 session — connection lifecycle as a module singleton', () => {
  test('startSession opens a socket, surfaces connecting then connected, and identifies on open', async () => {
    startSession({ url: server.url })
    expect(getSessionSnapshot().status).toBe('connecting')

    const conn = await server.waitForConnection()
    await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('connected'))
    await vi.waitFor(() => expect(conn.received.length).toBeGreaterThan(0))
    expect(JSON.parse(conn.received[0])).toMatchObject({
      type: 'identify',
      name: 'ryan',
      tool: 'electron'
    })
  })

  test('startSession is idempotent — a second call with the same url does NOT open a second socket', async () => {
    startSession({ url: server.url })
    startSession({ url: server.url })
    await server.waitForConnection()
    await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('connected'))
    expect(server.connectionCount()).toBe(1)
  })

  test('getSessionSnapshot returns a STABLE reference until state actually changes', async () => {
    startSession({ url: server.url })
    const a = getSessionSnapshot()
    expect(getSessionSnapshot()).toBe(a)
    await server.waitForConnection()
    await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('connected'))
    const c = getSessionSnapshot()
    expect(c).not.toBe(a)
    expect(getSessionSnapshot()).toBe(c)
  })

  test('subscribers are notified on state change and stop after unsubscribe', async () => {
    let hits = 0
    const unsub = subscribeSession(() => {
      hits += 1
    })
    const conn = await connect()
    expect(hits).toBeGreaterThan(0)
    const afterOpen = hits

    unsub()
    conn.send(serverMessage({ ts: '2026-05-24T00:00:01.000Z' }))
    await vi.waitFor(() => expect(getSessionSnapshot().messages.length).toBe(1))
    expect(hits).toBe(afterOpen)
  })
})

describe('#1537 session — message dedup + live-append subscription', () => {
  test('a chat message lands in messages; an identical duplicate (same ts|from|text) is deduped', async () => {
    const conn = await connect()
    conn.send(serverMessage())
    conn.send(serverMessage())
    await vi.waitFor(() => expect(getSessionSnapshot().messages.length).toBe(1))
    expect(getSessionSnapshot().messages.length).toBe(1)
  })

  test('subscribeLiveMessages fires once per newly-appended live message, not for dedup-dropped ones', async () => {
    const conn = await connect()
    const seen: string[] = []
    subscribeLiveMessages((m) => seen.push(m.text))
    conn.send(serverMessage({ text: 'one', ts: '2026-05-24T00:00:01.000Z' }))
    conn.send(serverMessage({ text: 'one', ts: '2026-05-24T00:00:01.000Z' }))
    conn.send(serverMessage({ text: 'two', ts: '2026-05-24T00:00:02.000Z' }))
    await vi.waitFor(() => expect(seen).toEqual(['one', 'two']))
  })
})

describe('#1537 session — history load-once + merge (no live-notify on history)', () => {
  test('history is fetched once at init and merged into messages; a duplicate start does NOT refetch', async () => {
    historyResponse = [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }]
    startSession({ url: server.url, apiBase: 'http://api', authToken: 'tok' })
    startSession({ url: server.url, apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    const startCalls = httpGet.mock.calls.length
    startSession({ url: server.url, apiBase: 'http://api', authToken: 'tok' })
    expect(httpGet.mock.calls.length).toBe(startCalls)
  })

  test('history-merge emits ZERO live-message append-events (only live frames notify)', async () => {
    historyResponse = [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }]
    const seen: string[] = []
    subscribeLiveMessages((m) => seen.push(m.text))
    startSession({ url: server.url, apiBase: 'http://api', authToken: 'tok' })
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    expect(seen).toEqual([])
  })
})

describe('#1537 session — reconnect with backoff', () => {
  test('on close, status goes disconnected and a reconnect opens a second socket', async () => {
    const conn = await connect()
    conn.drop()
    await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('disconnected'))
    expect(server.connectionCount()).toBe(1)

    await server.waitForConnection()
    expect(server.connectionCount()).toBe(2)
  }, 10000)

  test('reconnectTick increments on each disconnect→connect transition', async () => {
    const conn = await connect()
    expect(getSessionSnapshot().reconnectTick).toBe(0)

    conn.drop()
    await server.waitForConnection()
    await vi.waitFor(() => expect(getSessionSnapshot().reconnectTick).toBe(1))
  }, 10000)

  test('a transient ws close logs at warn, not error — keeps the e2e zero-console-error invariant robust under load', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const conn = await connect()
    conn.drop()
    await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('disconnected'))
    expect(errorSpy).not.toHaveBeenCalled()
    expect(warnSpy).toHaveBeenCalledWith('bpmux ws: closed, reconnecting', 1006, expect.any(String))
    errorSpy.mockRestore()
    warnSpy.mockRestore()
  })
})

describe('#1319 session — reconnect re-fetches history to recover the disconnect gap', () => {
  test('on reconnect, history is re-fetched so a message sent during the gap appears after recovery', async () => {
    historyResponse = [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }]
    const conn = await connect('http://api', 'tok')
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )
    const callsBeforeDrop = httpGet.mock.calls.length

    conn.drop()
    await vi.waitFor(() => expect(getSessionSnapshot().status).toBe('disconnected'))

    historyResponse = [
      { from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' },
      { from: 'designer', text: 'sent-during-gap', ts: '2026-05-28T00:01:00.000Z' }
    ]

    await server.waitForConnection()
    await vi.waitFor(() => expect(httpGet.mock.calls.length).toBeGreaterThan(callsBeforeDrop))
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'sent-during-gap')).toBe(true)
    )
  }, 10000)

  test('on reconnect-refetch, a duplicate pre-gap message (already in state) does not appear twice', async () => {
    historyResponse = [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }]
    const conn = await connect('http://api', 'tok')
    await vi.waitFor(() =>
      expect(getSessionSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )
    const callsBeforeDrop = httpGet.mock.calls.length

    conn.drop()
    await server.waitForConnection()
    await vi.waitFor(() => expect(httpGet.mock.calls.length).toBeGreaterThan(callsBeforeDrop))

    const preGapCount = getSessionSnapshot().messages.filter((m) => m.text === 'pre-gap').length
    expect(preGapCount).toBe(1)
  }, 10000)
})

describe('#1618 session — board-refresh via the board_invalidated state-delta', () => {
  test('a board_invalidated state-delta bumps boardInvalidateTick (the structured board-refresh trigger)', async () => {
    const conn = await connect()
    expect(getSessionSnapshot().boardInvalidateTick).toBe(0)
    conn.send(
      serverMessage({
        from: 'bpmux',
        text: '',
        message_type: 'state-delta',
        state_delta: { domain: 'board_invalidated', reason: 'board changed' }
      })
    )
    await vi.waitFor(() => expect(getSessionSnapshot().boardInvalidateTick).toBe(1))
  })

  test('a board_invalidated state-delta leaves the mirror reactions/ci/roster untouched', async () => {
    const conn = await connect()
    const before = getSessionSnapshot().mirror
    conn.send(
      serverMessage({
        from: 'bpmux',
        text: '',
        message_type: 'state-delta',
        state_delta: { domain: 'board_invalidated', reason: 'board changed' }
      })
    )
    await vi.waitFor(() => expect(getSessionSnapshot().boardInvalidateTick).toBe(1))
    expect(getSessionSnapshot().mirror).toBe(before)
  })
})
