import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { failure, success } from '../lib/result'
import { useMessageHistory } from './useMessageHistory'
import type { ChatMessage } from '../transport/types'

vi.mock('../api/http', async () => {
  const actual = await vi.importActual<typeof import('../api/http')>('../api/http')
  return {
    ...actual,
    http: {
      get: vi.fn(),
      post: actual.http.post,
      put: actual.http.put,
      delete: actual.http.delete
    }
  }
})

import { http } from '../api/http'

const live = (i: number): ChatMessage => ({
  id: `live-${i}`,
  from: 'ryan',
  text: `live ${i}`,
  ts: `2026-05-03T03:${String(50 + i).padStart(2, '0')}:00.000Z`,
  to: null,
  addressee: null,
  messageType: null
})

beforeEach(() => {
  vi.mocked(http.get).mockReset()
})

afterEach(() => {
  vi.restoreAllMocks()
})

const makeAsyncResult = <S, F>(result: ReturnType<typeof success<S, F>> | ReturnType<typeof failure<F, S>>) => ({
  value: Promise.resolve(result),
  map: vi.fn(),
  andThen: vi.fn(),
  or: vi.fn(),
  onSuccess: vi.fn(),
  onFailure: vi.fn(),
  either: vi.fn(),
  mapEither: vi.fn()
})

describe('#1359 useMessageHistory — Result-chain boundary behavior', () => {
  test('http.get failure flips hasError=true via Result.failure branch (not raw throw)', async () => {
    vi.mocked(http.get).mockReturnValueOnce(
      makeAsyncResult(failure({ kind: 'http', status: 500, body: 'oops' })) as never
    )
    const { result } = renderHook(() =>
      useMessageHistory({
        liveMessages: [live(0)],
        apiBase: 'http://test',
        authToken: 't0k'
      })
    )
    expect(result.current.hasError).toBe(false)
    await act(async () => {
      await result.current.loadOlder()
    })
    expect(result.current.hasError).toBe(true)
    expect(result.current.isLoadingOlder).toBe(false)
  })

  test('http.get success with empty payload flips hasMoreOlder=false', async () => {
    vi.mocked(http.get).mockReturnValueOnce(
      makeAsyncResult(success([])) as never
    )
    const { result } = renderHook(() =>
      useMessageHistory({
        liveMessages: [live(0)],
        apiBase: 'http://test',
        authToken: 't0k'
      })
    )
    await act(async () => {
      await result.current.loadOlder()
    })
    expect(result.current.hasMoreOlder).toBe(false)
    expect(result.current.hasError).toBe(false)
  })

  test('http.get success with rows prepends history without raising', async () => {
    vi.mocked(http.get).mockReturnValueOnce(
      makeAsyncResult(
        success([
          {
            from: 'pm',
            text: 'older message',
            ts: '2026-05-03T03:00:00.000Z',
            to: null,
            addressee: null,
            message_type: null
          }
        ])
      ) as never
    )
    const { result } = renderHook(() =>
      useMessageHistory({
        liveMessages: [live(0)],
        apiBase: 'http://test',
        authToken: 't0k'
      })
    )
    await act(async () => {
      await result.current.loadOlder()
    })
    expect(result.current.combinedMessages.length).toBeGreaterThanOrEqual(2)
    expect(result.current.hasError).toBe(false)
    expect(
      result.current.combinedMessages.some((m) => m.text === 'older message')
    ).toBe(true)
  })
})
