import { useCallback, useEffect, useState } from 'react'
import { describeHttpError } from '../api/http'
import { getBoard } from '../api/boardClient'
import { MOCK_BOARD_RESPONSE } from './mockBoard'
import { useRefetchOnReconnect } from './useRefetchOnReconnect'
import type { BoardResponse } from './types'

const POLL_INTERVAL_MS = 5 * 60_000
const RETRY_DELAYS_MS = [0, 200, 500] as const

export interface UseBoardFetchOptions {
  boardInvalidateTick: number
  apiBase: string | undefined
  authToken: string | undefined
}

export interface UseBoardFetchResult {
  data: BoardResponse | null
  setData: React.Dispatch<React.SetStateAction<BoardResponse | null>>
  loading: boolean
  error: string | null
  setError: React.Dispatch<React.SetStateAction<string | null>>
  usingMock: boolean
  refetch: () => void
}

export const useBoardFetch = ({
  boardInvalidateTick,
  apiBase,
  authToken
}: UseBoardFetchOptions): UseBoardFetchResult => {
  const [data, setData] = useState<BoardResponse | null>(null)
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)
  const [usingMock, setUsingMock] = useState<boolean>(false)

  const fetchBoard = useCallback(
    (signal?: AbortSignal): void => {
      if (apiBase === undefined) {
        setData(MOCK_BOARD_RESPONSE)
        setUsingMock(true)
        setError(null)
        setLoading(false)
        return
      }
      let attempt = 0
      const tryFetch = (): void => {
        getBoard({ apiBase, authToken }, signal)
          .onSuccess((board) => {
            if (signal?.aborted) return
            setData(board)
            setUsingMock(false)
            setError(null)
            setLoading(false)
          })
          .onFailure((reason) => {
            if (signal?.aborted) return
            attempt += 1
            if (attempt < RETRY_DELAYS_MS.length) {
              setTimeout(tryFetch, RETRY_DELAYS_MS[attempt])
              return
            }
            const message = describeHttpError(reason)
            console.error('bpmux board: fetch failed, falling back to mock', message)
            setData((prev) => prev ?? MOCK_BOARD_RESPONSE)
            setUsingMock((prev) => prev || !prev)
            setError(message)
            setLoading(false)
          })
      }
      tryFetch()
    },
    [apiBase, authToken]
  )

  useEffect(() => {
    const controller = new AbortController()
    const initial = setTimeout(() => fetchBoard(controller.signal), 0)
    const id = setInterval(() => fetchBoard(), POLL_INTERVAL_MS)
    return () => {
      controller.abort()
      clearTimeout(initial)
      clearInterval(id)
    }
  }, [fetchBoard])

  useRefetchOnReconnect(fetchBoard)

  useEffect(() => {
    if (boardInvalidateTick === 0) return
    const id = setTimeout(() => fetchBoard(), 0)
    return () => clearTimeout(id)
  }, [boardInvalidateTick, fetchBoard])

  const refetch = useCallback((): void => {
    fetchBoard()
  }, [fetchBoard])

  return { data, setData, loading, error, setError, usingMock, refetch }
}
