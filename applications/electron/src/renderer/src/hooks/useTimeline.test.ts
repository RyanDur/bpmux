import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'
import { useTimeline } from './useTimeline'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const fetchSpy = vi.fn()

const timelineCalls = (): number =>
  fetchSpy.mock.calls.filter((c) => String(c[0]).includes('/timeline')).length

const bumpBoard = (): void =>
  dispatchSession(
    mirrorDeltaReceived({ domain: 'board_invalidated', reason: 'board changed' }, '2026-01-01T00:00:00Z')
  )

beforeEach(() => {
  fetchSpy.mockReset()
  fetchSpy.mockResolvedValue({
    ok: true,
    status: 200,
    json: async () => [],
    text: async () => ''
  } as unknown as Response)
  vi.stubGlobal('fetch', fetchSpy)
})

afterEach(() => {
  resetSessionForTest()
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})

describe('#1456 useTimeline — refetch survives the ingest-strip (tick replaces marker-scan)', () => {
  const props = {
    apiBase: 'http://127.0.0.1:9',
    authToken: 'tok',
    storyNumber: 700
  }

  test('a board-invalidate tick increment triggers a timeline refetch (silent-regression guard)', async () => {
    renderHook((p: typeof props) => useTimeline(p), { initialProps: props })
    await waitFor(() => expect(timelineCalls()).toBeGreaterThanOrEqual(1))
    const before = timelineCalls()

    act(() => {
      bumpBoard()
    })

    await waitFor(() => expect(timelineCalls()).toBe(before + 1))
  })

  test('no board-invalidate bump does not refetch beyond the mount fetch', async () => {
    const { rerender } = renderHook((p: typeof props) => useTimeline(p), { initialProps: props })
    await waitFor(() => expect(timelineCalls()).toBeGreaterThanOrEqual(1))
    const before = timelineCalls()

    rerender(props)

    expect(timelineCalls()).toBe(before)
  })
})
