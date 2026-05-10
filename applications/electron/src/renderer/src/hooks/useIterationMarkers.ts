import { useEffect, useRef, useState } from 'react'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'

export interface IterationMarker {
  id: number
  theme_name: string
  position: number
  created_at: string
}

interface Args {
  apiBase: string | undefined
  authToken: string | undefined
  /** #1255 hot-update: incoming chat messages. The hook scans for
   * BOARD_INVALIDATE_MARKER and refetches /api/iteration_markers so
   * sister-roles see PM-created markers without a page reload. */
  messages?: ChatMessage[]
}

/// #1255 pure helper: given the seen-message count from the previous
/// render and the current messages array, returns true iff a NEW
/// (post-prevSeenCount) message carries the BOARD_INVALIDATE_MARKER
/// text. Pure-fn lets vitest exercise the gate logic without a React
/// renderer.
export const shouldRefetchMarkers = (
  messages: ChatMessage[] | undefined,
  prevSeenCount: number
): boolean => {
  if (messages === undefined) return false
  if (messages.length <= prevSeenCount) return false
  const fresh = messages.slice(prevSeenCount)
  return fresh.some((m) => m.text === BOARD_INVALIDATE_MARKER)
}

export const useIterationMarkers = ({ apiBase, authToken, messages }: Args): IterationMarker[] => {
  const [markers, setMarkers] = useState<IterationMarker[]>([])
  const seenMessageCountRef = useRef<number>(messages?.length ?? 0)

  useEffect(() => {
    if (!apiBase) return
    const controller = new AbortController()
    fetch(`${apiBase}/api/iteration_markers`, {
      headers: { Authorization: `Bearer ${authToken ?? ''}` },
      signal: controller.signal
    })
      .then(async (res) => {
        if (!res.ok) return
        const data = (await res.json()) as IterationMarker[]
        setMarkers(data)
      })
      .catch((err) => {
        if ((err as Error)?.name === 'AbortError') return
      })
    return () => controller.abort()
  }, [apiBase, authToken])

  useEffect(() => {
    if (!apiBase) return
    const prev = seenMessageCountRef.current
    const refetch = shouldRefetchMarkers(messages, prev)
    seenMessageCountRef.current = messages?.length ?? prev
    if (!refetch) return
    const controller = new AbortController()
    fetch(`${apiBase}/api/iteration_markers`, {
      headers: { Authorization: `Bearer ${authToken ?? ''}` },
      signal: controller.signal
    })
      .then(async (res) => {
        if (!res.ok) return
        const data = (await res.json()) as IterationMarker[]
        setMarkers(data)
      })
      .catch((err) => {
        if ((err as Error)?.name === 'AbortError') return
      })
    return () => controller.abort()
  }, [apiBase, authToken, messages])

  return markers
}
