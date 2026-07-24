import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'
import { useMessageHistory } from '../useMessageHistory'
import {
  BrowserWebSocket,
  startTestWsServer,
  type TestWsConnection,
  type TestWsServer
} from '../../../../../../test/wsTestServer'
import { startConnection, getStateSnapshot, resetStateForTest } from '@state'
import type { ChatMessage } from '@transport/types'

const live = (i: number): ChatMessage => ({
  id: `live-${i}`,
  from: 'owner',
  text: `live ${i}`,
  ts: `2026-05-03T03:${String(50 + i).padStart(2, '0')}:00.000Z`,
  to: null,
  addressee: null,
  messageType: null
})

let server: TestWsServer
let conn: TestWsConnection

const pageRequestId = (): number | null => {
  for (let i = conn.received.length - 1; i >= 0; i -= 1) {
    try {
      const frame = JSON.parse(conn.received[i]) as { type?: string; request_id?: number }
      if (frame.type === 'history_page') return frame.request_id ?? null
    } catch {
      continue
    }
  }
  return null
}

const props = { liveMessages: [live(0)], apiBase: 'http://test' }

beforeEach(async () => {
  vi.stubGlobal('WebSocket', BrowserWebSocket)
  server = await startTestWsServer()
  resetStateForTest()
  startConnection({ url: server.url })
  conn = await server.waitForConnection()
  await waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
})

afterEach(async () => {
  resetStateForTest()
  await server.close()
  vi.unstubAllGlobals()
})

describe('useMessageHistory — WS older-history boundary behavior', () => {
  test('a socket drop mid-request flips hasError=true (request rejected, not a raw throw)', async () => {
    const { result } = renderHook(() => useMessageHistory(props))
    expect(result.current.hasError).toBe(false)

    let pending!: Promise<void>
    act(() => {
      pending = result.current.loadOlder()
    })
    await waitFor(() => expect(pageRequestId()).not.toBeNull())

    await act(async () => {
      conn.drop()
      await pending
    })

    expect(result.current.hasError).toBe(true)
    expect(result.current.isLoadingOlder).toBe(false)
  })

  test('an empty page reply flips hasMoreOlder=false', async () => {
    const { result } = renderHook(() => useMessageHistory(props))

    let pending!: Promise<void>
    act(() => {
      pending = result.current.loadOlder()
    })
    await waitFor(() => expect(pageRequestId()).not.toBeNull())

    await act(async () => {
      conn.send(
        JSON.stringify({ type: 'history_page_loaded', request_id: pageRequestId(), messages: [] })
      )
      await pending
    })

    expect(result.current.hasMoreOlder).toBe(false)
    expect(result.current.hasError).toBe(false)
  })

  test('a page reply with rows prepends older history without raising', async () => {
    const { result } = renderHook(() => useMessageHistory(props))

    let pending!: Promise<void>
    act(() => {
      pending = result.current.loadOlder()
    })
    await waitFor(() => expect(pageRequestId()).not.toBeNull())

    await act(async () => {
      conn.send(
        JSON.stringify({
          type: 'history_page_loaded',
          request_id: pageRequestId(),
          messages: [
            {
              from: 'pm',
              text: 'older message',
              ts: '2026-05-03T03:00:00.000Z',
              to: null,
              addressee: null,
              message_type: null
            }
          ]
        })
      )
      await pending
    })

    expect(result.current.combinedMessages.length).toBeGreaterThanOrEqual(2)
    expect(result.current.hasError).toBe(false)
    expect(result.current.combinedMessages.some((m) => m.text === 'older message')).toBe(true)
  })
})
