import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'
import { useTimeline } from './useTimeline'
import {
  BrowserWebSocket,
  startTestWsServer,
  type TestWsConnection,
  type TestWsServer
} from '../../../../test/wsTestServer'
import { startConnection, getStateSnapshot, resetStateForTest } from '../state'
import { dispatchState } from '../state/store'
import { mirrorDeltaReceived } from '../state/actions'
const props = {
  apiBase: 'http://backend.invalid',
  storyId: 700
}

const bumpBoard = (): void =>
  dispatchState(
    mirrorDeltaReceived(
      { domain: 'board_invalidated', reason: 'board changed' },
      '2026-01-01T00:00:00Z'
    )
  )

describe('useTimeline — re-hydrates over WS when the board changes (tick replaces marker-scan)', () => {
  let server: TestWsServer
  let conn: TestWsConnection

  const timelineFrames = (): number =>
    conn.received.filter((r) => {
      try {
        return (JSON.parse(r) as { type?: string }).type === 'timeline_hydrate'
      } catch {
        return false
      }
    }).length

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

  test('a board-invalidate tick increment triggers a timeline re-hydrate frame (silent-regression guard)', async () => {
    renderHook((p: typeof props) => useTimeline(p), { initialProps: props })
    await waitFor(() => expect(timelineFrames()).toBeGreaterThanOrEqual(1))
    const before = timelineFrames()

    act(() => {
      bumpBoard()
    })

    await waitFor(() => expect(timelineFrames()).toBe(before + 1))
  })

  test('classifies the events the backend serves for this story', async () => {
    const { result } = renderHook((p: typeof props) => useTimeline(p), { initialProps: props })
    await waitFor(() => expect(timelineFrames()).toBeGreaterThanOrEqual(1))

    act(() => {
      conn.send(
        JSON.stringify({
          type: 'timeline_hydrated',
          story_id: 700,
          events: [
            { event: 'filed', actor: 'pm', ts: '2026-04-20T10:00:00Z' },
            { event: 'labeled', actor: 'dev', ts: '2026-04-21T10:00:00Z', label: 'delivered' }
          ]
        })
      )
    })

    await waitFor(() => expect(result.current.events?.length).toBe(2))
    expect(result.current.events?.[0].gate).toBe('filed')
    expect(result.current.events?.[1].gate).toBe('delivered')
    expect(result.current.loading).toBe(false)
  })
})

describe('useTimeline — offline (mock) mode', () => {
  afterEach(() => {
    resetStateForTest()
  })

  test('renders empty (not loading) when offline (apiBase undefined)', () => {
    const { result } = renderHook(() => useTimeline({ apiBase: undefined, storyId: 700 }))
    expect(result.current.events).toEqual([])
    expect(result.current.loading).toBe(false)
  })
})
