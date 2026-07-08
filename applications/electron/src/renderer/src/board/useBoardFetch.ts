import { useCallback, useState } from 'react'
import { useStories } from '../transport/state'
import { useRefetchOnReconnect } from './useRefetchOnReconnect'
import type { Story } from './types'

export interface UseBoardFetchOptions {
  apiBase: string | undefined
}

export interface UseBoardFetchResult {
  data: Story[] | null
  setData: React.Dispatch<React.SetStateAction<Story[] | null>>
  loading: boolean
  error: string | null
  setError: React.Dispatch<React.SetStateAction<string | null>>
  refetch: () => void
}

export const useBoardFetch = ({ apiBase }: UseBoardFetchOptions): UseBoardFetchResult => {
  const stories = useStories()
  const [data, setData] = useState<Story[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  const offline = apiBase === undefined
  const loading = !offline && data === null && error === null

  const refetch = useCallback((): void => {
    setData(stories)
    setError(null)
  }, [stories])

  useRefetchOnReconnect(refetch)

  const [lastAppliedStories, setLastAppliedStories] = useState<Story[] | null>(null)
  if (!offline && stories !== lastAppliedStories) {
    setLastAppliedStories(stories)
    setData(stories)
    setError(null)
  }

  return { data, setData, loading, error, setError, refetch }
}
