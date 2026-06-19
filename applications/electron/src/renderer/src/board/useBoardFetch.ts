import { useCallback, useEffect, useState } from 'react'
import { requestBoardHydrate, useBoardSnapshot, useSessionId } from '../transport/session'
import { useRefetchOnReconnect } from './useRefetchOnReconnect'
import type { BoardResponse } from './types'

export interface UseBoardFetchOptions {
  apiBase: string | undefined
}

export interface UseBoardFetchResult {
  data: BoardResponse | null
  setData: React.Dispatch<React.SetStateAction<BoardResponse | null>>
  loading: boolean
  error: string | null
  setError: React.Dispatch<React.SetStateAction<string | null>>
  refetch: () => void
}

export const useBoardFetch = ({ apiBase }: UseBoardFetchOptions): UseBoardFetchResult => {
  const boardSnapshot = useBoardSnapshot()
  const sessionId = useSessionId()
  const [data, setData] = useState<BoardResponse | null>(null)
  const [error, setError] = useState<string | null>(null)

  const offline = apiBase === undefined
  const loading = !offline && data === null && error === null

  const refetch = useCallback((): void => {
    if (offline) return
    requestBoardHydrate()
  }, [offline])

  useEffect(() => {
    if (offline || sessionId === null) return
    requestBoardHydrate()
  }, [offline, sessionId])

  useRefetchOnReconnect(refetch)

  const [lastAppliedSnapshot, setLastAppliedSnapshot] = useState<BoardResponse | null>(null)
  if (!offline && boardSnapshot !== null && boardSnapshot !== lastAppliedSnapshot) {
    setLastAppliedSnapshot(boardSnapshot)
    setData(boardSnapshot)
    setError(null)
  }

  return { data, setData, loading, error, setError, refetch }
}
