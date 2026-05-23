import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { useTimeline } from './useTimeline'

const fetchSpy = vi.fn()

const timelineCalls = (): number =>
  fetchSpy.mock.calls.filter((c) => String(c[0]).includes('/timeline')).length

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
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})

describe('#1456 useTimeline — refetch survives the ingest-strip (tick replaces marker-scan)', () => {
  const props = {
    apiBase: 'http://127.0.0.1:9',
    authToken: 'tok',
    storyNumber: 700,
    boardInvalidateTick: 0
  }

  test('a board-invalidate tick increment triggers a timeline refetch (silent-regression guard)', async () => {
    const { rerender } = renderHook((p: typeof props) => useTimeline(p), { initialProps: props })
    await waitFor(() => expect(timelineCalls()).toBeGreaterThanOrEqual(1))
    const before = timelineCalls()

    rerender({ ...props, boardInvalidateTick: 1 })

    await waitFor(() => expect(timelineCalls()).toBe(before + 1))
  })

  test('tick at 0 does not refetch beyond the mount fetch', async () => {
    const { rerender } = renderHook((p: typeof props) => useTimeline(p), { initialProps: props })
    await waitFor(() => expect(timelineCalls()).toBeGreaterThanOrEqual(1))
    const before = timelineCalls()

    rerender({ ...props, boardInvalidateTick: 0 })

    expect(timelineCalls()).toBe(before)
  })
})
