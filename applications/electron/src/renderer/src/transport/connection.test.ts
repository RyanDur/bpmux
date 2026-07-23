import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import {
  BrowserWebSocket,
  startTestWsServer,
  type TestWsConnection,
  type TestWsServer
} from '../../../../test/wsTestServer'

import {
  startConnection,
  getStateSnapshot,
  subscribeState,
  subscribeLiveMessages,
  resetStateForTest
} from '../state'

const serverMessage = (over: Record<string, unknown> = {}): string =>
  JSON.stringify({
    type: 'message',
    from: 'pm',
    text: 'hello',
    ts: '2026-05-24T00:00:00.000Z',
    ...over
  })

const WELCOME = JSON.stringify({ type: 'welcome' })

const historyHydrateCount = (conn: TestWsConnection): number =>
  conn.received.filter((r) => {
    try {
      return (JSON.parse(r) as { type?: string }).type === 'history_hydrate'
    } catch {
      return false
    }
  }).length

const serveHistory = async (
  conn: TestWsConnection,
  messages: Record<string, unknown>[]
): Promise<void> => {
  conn.send(WELCOME)
  await vi.waitFor(() => expect(historyHydrateCount(conn)).toBeGreaterThan(0))
  conn.send(JSON.stringify({ type: 'history_hydrated', messages }))
}

let server: TestWsServer

const connect = async (apiBase?: string, authToken?: string): Promise<TestWsConnection> => {
  startConnection({ url: server.url, apiBase, authToken })
  const conn = await server.waitForConnection()
  await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
  return conn
}

beforeEach(async () => {
  vi.stubGlobal('WebSocket', BrowserWebSocket)
  server = await startTestWsServer()
  resetStateForTest()
})

afterEach(async () => {
  resetStateForTest()
  await server.close()
  vi.unstubAllGlobals()
})

describe('connection — lifecycle as a module singleton', () => {
  test('startConnection opens a socket, surfaces connecting then connected, and identifies on open', async () => {
    startConnection({ url: server.url })
    expect(getStateSnapshot().status).toBe('connecting')

    const conn = await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    await vi.waitFor(() => expect(conn.received.length).toBeGreaterThan(0))
    expect(JSON.parse(conn.received[0])).toMatchObject({
      type: 'identify',
      name: 'owner',
      tool: 'electron'
    })
  })

  test('identify carries owner_nonce when the config supplies the launch nonce (owner-anchor carriage)', async () => {
    startConnection({
      url: server.url,
      apiBase: 'http://api',
      authToken: 'tok',
      ownerNonce: 'launch-nonce-xyz'
    })
    const conn = await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    await vi.waitFor(() => expect(conn.received.length).toBeGreaterThan(0))
    expect(JSON.parse(conn.received[0])).toMatchObject({
      type: 'identify',
      name: 'owner',
      tool: 'electron',
      owner_nonce: 'launch-nonce-xyz'
    })
  })

  test('identify omits owner_nonce entirely when the config has none (old-config / non-electron surface)', async () => {
    startConnection({ url: server.url })
    const conn = await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    await vi.waitFor(() => expect(conn.received.length).toBeGreaterThan(0))
    const frame = JSON.parse(conn.received[0]) as Record<string, unknown>
    expect(frame).toMatchObject({ type: 'identify', name: 'owner', tool: 'electron' })
    expect('owner_nonce' in frame).toBe(false)
  })

  test('startConnection is idempotent — a second call with the same url does NOT open a second socket', async () => {
    startConnection({ url: server.url })
    startConnection({ url: server.url })
    await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    expect(server.connectionCount()).toBe(1)
  })

  test('getStateSnapshot returns a STABLE reference until state actually changes', async () => {
    startConnection({ url: server.url })
    const a = getStateSnapshot()
    expect(getStateSnapshot()).toBe(a)
    await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    const c = getStateSnapshot()
    expect(c).not.toBe(a)
    expect(getStateSnapshot()).toBe(c)
  })

  test('subscribers are notified on state change and stop after unsubscribe', async () => {
    let hits = 0
    const unsub = subscribeState(() => {
      hits += 1
    })
    const conn = await connect()
    expect(hits).toBeGreaterThan(0)
    const afterOpen = hits

    unsub()
    conn.send(serverMessage({ ts: '2026-05-24T00:00:01.000Z' }))
    await vi.waitFor(() => expect(getStateSnapshot().messages.length).toBe(1))
    expect(hits).toBe(afterOpen)
  })
})

describe('connection — message dedup + live-append subscription', () => {
  test('a chat message lands in messages; an identical duplicate (same ts|from|text) is deduped', async () => {
    const conn = await connect()
    conn.send(serverMessage())
    conn.send(serverMessage())
    await vi.waitFor(() => expect(getStateSnapshot().messages.length).toBe(1))
    expect(getStateSnapshot().messages.length).toBe(1)
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

describe('connection — history hydrate-on-identify + merge (no live-notify on history)', () => {
  test('history hydrates over WS on identify and merges; a duplicate start does NOT re-hydrate', async () => {
    const conn = await connect('http://api', 'tok')
    await serveHistory(conn, [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }])
    await vi.waitFor(() =>
      expect(getStateSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    const hydrateFrames = historyHydrateCount(conn)
    startConnection({ url: server.url, apiBase: 'http://api', authToken: 'tok' })
    expect(historyHydrateCount(conn)).toBe(hydrateFrames)
  })

  test('history-merge emits ZERO live-message append-events (only live frames notify)', async () => {
    const seen: string[] = []
    subscribeLiveMessages((m) => seen.push(m.text))
    const conn = await connect('http://api', 'tok')
    await serveHistory(conn, [{ from: 'pm', text: 'old', ts: '2026-05-23T00:00:00.000Z' }])
    await vi.waitFor(() =>
      expect(getStateSnapshot().messages.some((m) => m.text === 'old')).toBe(true)
    )
    expect(seen).toEqual([])
  })
})

describe('connection — reconnect with backoff', () => {
  test('on close, status goes disconnected and a reconnect opens a second socket', async () => {
    const conn = await connect()
    conn.drop()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('disconnected'))
    expect(server.connectionCount()).toBe(1)

    await server.waitForConnection()
    expect(server.connectionCount()).toBe(2)
  }, 10000)

  test('reconnectTick increments on each disconnect→connect transition', async () => {
    const conn = await connect()
    expect(getStateSnapshot().reconnectTick).toBe(0)

    conn.drop()
    await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().reconnectTick).toBe(1))
  }, 10000)

  test('a transient ws close logs at warn, not error — keeps the e2e zero-console-error invariant robust under load', async () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const conn = await connect()
    conn.drop()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('disconnected'))
    expect(errorSpy).not.toHaveBeenCalled()
    expect(warnSpy).toHaveBeenCalledWith('bpmux ws: closed, reconnecting', 1006, expect.any(String))
    errorSpy.mockRestore()
    warnSpy.mockRestore()
  })
})

describe('connection — reconnect re-hydrates history to recover the disconnect gap', () => {
  test('on reconnect, history re-hydrates so a message sent during the gap appears after recovery', async () => {
    const conn = await connect('http://api', 'tok')
    await serveHistory(conn, [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }])
    await vi.waitFor(() =>
      expect(getStateSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )

    conn.drop()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('disconnected'))

    const conn2 = await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    await serveHistory(conn2, [
      { from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' },
      { from: 'designer', text: 'sent-during-gap', ts: '2026-05-28T00:01:00.000Z' }
    ])
    await vi.waitFor(() =>
      expect(getStateSnapshot().messages.some((m) => m.text === 'sent-during-gap')).toBe(true)
    )
  }, 10000)

  test('on reconnect-rehydrate, a duplicate pre-gap message (already in state) does not appear twice', async () => {
    const conn = await connect('http://api', 'tok')
    await serveHistory(conn, [{ from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' }])
    await vi.waitFor(() =>
      expect(getStateSnapshot().messages.some((m) => m.text === 'pre-gap')).toBe(true)
    )

    conn.drop()
    const conn2 = await server.waitForConnection()
    await vi.waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    await serveHistory(conn2, [
      { from: 'pm', text: 'pre-gap', ts: '2026-05-28T00:00:00.000Z' },
      { from: 'designer', text: 'post-recovery', ts: '2026-05-28T00:02:00.000Z' }
    ])
    await vi.waitFor(() =>
      expect(getStateSnapshot().messages.some((m) => m.text === 'post-recovery')).toBe(true)
    )

    const preGapCount = getStateSnapshot().messages.filter((m) => m.text === 'pre-gap').length
    expect(preGapCount).toBe(1)
  }, 10000)
})

describe('connection — board-refresh via the board_invalidated state-delta', () => {
  test('a board_invalidated state-delta bumps boardInvalidateTick (the structured board-refresh trigger)', async () => {
    const conn = await connect()
    expect(getStateSnapshot().boardInvalidateTick).toBe(0)
    conn.send(
      serverMessage({
        from: 'bpmux',
        text: '',
        message_type: 'state-delta',
        state_delta: { domain: 'board_invalidated', reason: 'board changed' }
      })
    )
    await vi.waitFor(() => expect(getStateSnapshot().boardInvalidateTick).toBe(1))
  })

  test('a board_invalidated state-delta leaves the mirror reactions/ci/roster untouched', async () => {
    const conn = await connect()
    const before = getStateSnapshot().mirror
    conn.send(
      serverMessage({
        from: 'bpmux',
        text: '',
        message_type: 'state-delta',
        state_delta: { domain: 'board_invalidated', reason: 'board changed' }
      })
    )
    await vi.waitFor(() => expect(getStateSnapshot().boardInvalidateTick).toBe(1))
    expect(getStateSnapshot().mirror).toBe(before)
  })
})
