import { useCallback, useEffect, useRef, useState } from 'react'
import { describeHttpError } from '../api/http'
import { getBoard } from '../api/boardClient'
import {
  BOARD_INVALIDATE_MARKER,
  type ChatMessage,
  type ConnectionStatus
} from '../transport/types'
import { MOCK_BOARD_RESPONSE } from './mockBoard'
import { useRefetchOnReconnect } from './useRefetchOnReconnect'
import type { BoardResponse } from './types'

const POLL_INTERVAL_MS = 5 * 60_000
const RETRY_DELAYS_MS = [0, 200, 500] as const

export interface UseBoardFetchOptions {
  messages: ChatMessage[]
  wsStatus: ConnectionStatus | undefined
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
  messages,
  wsStatus,
  apiBase,
  authToken
}: UseBoardFetchOptions): UseBoardFetchResult => {
  const [data, setData] = useState<BoardResponse | null>(null)
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)
  const [usingMock, setUsingMock] = useState<boolean>(false)
  const seenMessageCountRef = useRef<number>(messages.length)

  const fetchBoard = useCallback(
    async (signal?: AbortSignal): Promise<void> => {
      if (apiBase === undefined) {
        await Promise.resolve()
        if (signal?.aborted) return
        setData(MOCK_BOARD_RESPONSE)
        setUsingMock(true)
        setError(null)
        setLoading(false)
        return
      }
      let lastErr: unknown = null
      for (const delay of RETRY_DELAYS_MS) {
        if (signal?.aborted) return
        if (delay > 0) {
          await new Promise<void>((resolve) => setTimeout(resolve, delay))
          if (signal?.aborted) return
        }
        const fetchResult = await getBoard({ apiBase, authToken }, signal).value
        if (fetchResult.kind === 'success') {
          setData(fetchResult.value)
          setUsingMock(false)
          setError(null)
          setLoading(false)
          return
        }
        if (signal?.aborted) return
        lastErr = new Error(describeHttpError(fetchResult.reason))
      }
      console.error('bpmux board: fetch failed, falling back to mock', lastErr)
      setData((prev) => prev ?? MOCK_BOARD_RESPONSE)
      setUsingMock((prev) => prev || !prev)
      setError(lastErr instanceof Error ? lastErr.message : String(lastErr))
      setLoading(false)
    },
    [apiBase, authToken]
  )

  useEffect(() => {
    const controller = new AbortController()
    /* eslint-disable-next-line react-hooks/set-state-in-effect */
    void fetchBoard(controller.signal)
    const id = setInterval(() => {
      void fetchBoard()
    }, POLL_INTERVAL_MS)
    return () => {
      controller.abort()
      clearInterval(id)
    }
  }, [fetchBoard])

  useRefetchOnReconnect(wsStatus ?? 'disconnected', () => {
    void fetchBoard()
  })

  useEffect(() => {
    const prev = seenMessageCountRef.current
    if (messages.length <= prev) {
      seenMessageCountRef.current = messages.length
      return
    }
    const fresh = messages.slice(prev)
    seenMessageCountRef.current = messages.length
    if (fresh.some((m) => m.text === BOARD_INVALIDATE_MARKER)) {
      void fetchBoard()
    }
  }, [messages, fetchBoard])

  const refetch = useCallback((): void => {
    void fetchBoard()
  }, [fetchBoard])

  return { data, setData, loading, error, setError, usingMock, refetch }
}
