import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'

const getBoard = vi.fn()

vi.mock('../api/boardClient', () => ({
  getBoard: (...args: unknown[]) => getBoard(...args)
}))

import { useBoardFetch } from './useBoardFetch'
import type { BoardResponse } from './types'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const chainableSuccess = (): unknown => {
  const chain = {
    onSuccess: () => chain,
    onFailure: () => chain
  }
  return chain
}

const pushSnapshot = (snapshot: BoardResponse): void =>
  dispatchSession(
    mirrorDeltaReceived(
      { domain: 'board_snapshot', stories: snapshot.stories },
      snapshot.fetched_at
    )
  )

beforeEach(() => {
  getBoard.mockReset()
  getBoard.mockImplementation(() => chainableSuccess())
})

afterEach(() => {
  resetSessionForTest()
  vi.clearAllMocks()
})

describe('#1720 useBoardFetch — a pushed board-snapshot becomes the data with no refetch (render=f(substrate))', () => {
  const props = {
    apiBase: 'http://127.0.0.1:9',
    authToken: 'tok'
  }

  test('a board-snapshot push replaces board data directly without an /api/board refetch', async () => {
    const { result } = renderHook((p: typeof props) => useBoardFetch(p), {
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
    act(() => pushSnapshot(pushed))

    await waitFor(() =>
      expect(result.current.data?.stories?.[0]).toMatchObject({ number: 7, state: 'delivered' })
    )
    expect(getBoard.mock.calls.length).toBe(callsAfterMount)
  })
})
