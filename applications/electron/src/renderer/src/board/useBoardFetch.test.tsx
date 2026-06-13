import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'

const getBoard = vi.fn()

vi.mock('../api/boardClient', () => ({
  getBoard: (...args: unknown[]) => getBoard(...args)
}))

import { useBoardFetch } from './useBoardFetch'
import type { BoardResponse } from './types'

const chainableSuccess = (): unknown => {
  const chain = {
    onSuccess: () => chain,
    onFailure: () => chain
  }
  return chain
}

beforeEach(() => {
  getBoard.mockReset()
  getBoard.mockImplementation(() => chainableSuccess())
})

afterEach(() => vi.clearAllMocks())

describe('#1720 useBoardFetch — a pushed board-snapshot becomes the data with no refetch (render=f(substrate))', () => {
  const props = {
    boardSnapshot: null as BoardResponse | null,
    apiBase: 'http://127.0.0.1:9',
    authToken: 'tok'
  }

  test('a board-snapshot push replaces board data directly without an /api/board refetch', async () => {
    const { result, rerender } = renderHook((p: typeof props) => useBoardFetch(p), {
      initialProps: props
    })
    await waitFor(() => expect(getBoard).toHaveBeenCalled())
    const callsAfterMount = getBoard.mock.calls.length

    const pushed: BoardResponse = {
      stories: [
        {
          number: 7,
          title: 'pushed-live',
          state: 'delivered',
          labels: [],
          assignees: [],
          created_at: '2026-06-13T00:00:00Z',
          body: ''
        }
      ],
      fetched_at: '2026-06-13T00:00:00Z'
    }
    rerender({ ...props, boardSnapshot: pushed })

    await waitFor(() =>
      expect(result.current.data?.stories?.[0]).toMatchObject({ number: 7, state: 'delivered' })
    )
    expect(getBoard.mock.calls.length).toBe(callsAfterMount)
  })
})
