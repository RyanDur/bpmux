import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'

const getBoard = vi.fn()

vi.mock('../api/boardClient', () => ({
  getBoard: (...args: unknown[]) => getBoard(...args)
}))

import { useBoardFetch } from './useBoardFetch'

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

describe('#1456 useBoardFetch — refetch survives the ingest-strip (tick replaces marker-scan)', () => {
  const props = {
    boardInvalidateTick: 0,
    apiBase: 'http://127.0.0.1:9',
    authToken: 'tok'
  }

  test('a board-invalidate tick increment triggers a refetch (silent-regression guard)', async () => {
    const { rerender } = renderHook((p: typeof props) => useBoardFetch(p), {
      initialProps: props
    })
    await waitFor(() => expect(getBoard).toHaveBeenCalled())
    const callsAfterMount = getBoard.mock.calls.length

    rerender({ ...props, boardInvalidateTick: 1 })

    await waitFor(() => expect(getBoard.mock.calls.length).toBe(callsAfterMount + 1))
  })

  test('tick at 0 does not refetch beyond the mount fetch', async () => {
    const { rerender } = renderHook((p: typeof props) => useBoardFetch(p), {
      initialProps: props
    })
    await waitFor(() => expect(getBoard).toHaveBeenCalled())
    const callsAfterMount = getBoard.mock.calls.length

    rerender({ ...props, boardInvalidateTick: 0 })

    expect(getBoard.mock.calls.length).toBe(callsAfterMount)
  })
})
