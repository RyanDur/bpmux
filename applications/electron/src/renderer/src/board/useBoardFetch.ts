import { useCallback, useEffect, useState } from 'react'
import { describeHttpError } from '../api/http'
import { getBoard } from '../api/boardClient'
import { useBoardSnapshot } from '../transport/session'
import { useRefetchOnReconnect } from './useRefetchOnReconnect'
import type { BoardResponse } from './types'

const RETRY_DELAYS_MS = [0, 200, 500] as const

export interface UseBoardFetchOptions {
  apiBase: string | undefined
  authToken: string | undefined
}

export interface UseBoardFetchResult {
  data: BoardResponse | null
  setData: React.Dispatch<React.SetStateAction<BoardResponse | null>>
  loading: boolean
  error: string | null
  setError: React.Dispatch<React.SetStateAction<string | null>>
  refetch: () => void
}

export const useBoardFetch = ({
  apiBase,
  authToken
}: UseBoardFetchOptions): UseBoardFetchResult => {
  const boardSnapshot = useBoardSnapshot()
  const [data, setData] = useState<BoardResponse | null>(null)
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)

  const fetchBoard = useCallback(
    (signal?: AbortSignal): void => {
      if (apiBase === undefined) {
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
            console.warn('bpmux board: fetch failed', message)
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
    return () => {
      controller.abort()
      clearTimeout(initial)
    }
  }, [fetchBoard])

  useRefetchOnReconnect(fetchBoard)

  const [lastAppliedSnapshot, setLastAppliedSnapshot] = useState<BoardResponse | null>(null)
  if (
    apiBase !== undefined &&
    boardSnapshot !== null &&
    boardSnapshot !== undefined &&
    boardSnapshot !== lastAppliedSnapshot
  ) {
    setLastAppliedSnapshot(boardSnapshot)
    setData(boardSnapshot)
    setError(null)
    setLoading(false)
  }

  const refetch = useCallback((): void => {
    fetchBoard()
  }, [fetchBoard])

  return { data, setData, loading, error, setError, refetch }
}
