import { useCallback, useMemo, useState } from 'react'
import type { ChatMessage } from '../transport/types'

const DEFAULT_PAGE_SIZE = 200

interface HistoryMessage {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

export interface UseMessageHistoryOptions {
  liveMessages: ChatMessage[]
  apiBase?: string
  authToken?: string
  pageSize?: number
}

export interface UseMessageHistoryResult {
  combinedMessages: ChatMessage[]
  loadOlder: () => Promise<void>
  isLoadingOlder: boolean
  hasMoreOlder: boolean
  hasError: boolean
}

type FetchOk<T> = { ok: true; data: T }
type FetchErr = { ok: false }
type FetchResult<T> = FetchOk<T> | FetchErr

const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

const fetchPage = async (
  url: string,
  headers: Record<string, string>
): Promise<FetchResult<HistoryMessage[]>> => {
  try {
    const res = await fetch(url, { headers })
    if (!res.ok) {
      console.error('bpmux: loadOlder non-ok response', res.status)
      return { ok: false }
    }
    const data = (await res.json()) as HistoryMessage[]
    return { ok: true, data }
  } catch (err) {
    console.error('bpmux: loadOlder failed', err)
    return { ok: false }
  }
}

export const mergeOlderHistory = (
  prependedHistory: ChatMessage[],
  olderRows: HistoryMessage[],
  liveMessages: ChatMessage[]
): ChatMessage[] => {
  const chronological = [...olderRows].sort((a, b) => a.ts.localeCompare(b.ts))
  const seen = new Set(prependedHistory.map((m) => messageKey(m)))
  const liveKeys = new Set(liveMessages.map((m) => messageKey(m)))
  const out: ChatMessage[] = []
  let counter = 0
  for (const h of chronological) {
    const chat: ChatMessage = {
      id: `hist-older-${h.ts}-${counter++}`,
      from: h.from,
      text: h.text,
      ts: h.ts,
      to: h.to ?? null,
      addressee: h.addressee ?? null,
      messageType: h.message_type ?? null
    }
    const k = messageKey(chat)
    if (seen.has(k) || liveKeys.has(k)) continue
    seen.add(k)
    out.push(chat)
  }
  return [...out, ...prependedHistory]
}

export const useMessageHistory = ({
  liveMessages,
  apiBase,
  authToken,
  pageSize = DEFAULT_PAGE_SIZE
}: UseMessageHistoryOptions): UseMessageHistoryResult => {
  const [prependedHistory, setPrependedHistory] = useState<ChatMessage[]>([])
  const [isLoadingOlder, setIsLoadingOlder] = useState(false)
  const [hasMoreOlder, setHasMoreOlder] = useState(true)
  const [hasError, setHasError] = useState(false)

  const combinedMessages = useMemo(
    () => [...prependedHistory, ...liveMessages],
    [prependedHistory, liveMessages]
  )

  const loadOlder = useCallback(async (): Promise<void> => {
    if (!apiBase) return
    if (isLoadingOlder) return
    if (!hasMoreOlder) return

    const oldest = prependedHistory[0]?.ts ?? liveMessages[0]?.ts ?? null
    if (!oldest) return

    setIsLoadingOlder(true)
    const url = `${apiBase}/api/messages?chat_only=true&before=${encodeURIComponent(oldest)}&limit=${pageSize}`
    const headers: Record<string, string> = authToken
      ? { Authorization: `Bearer ${authToken}` }
      : {}
    const result = await fetchPage(url, headers)
    if (!result.ok) {
      setHasError(true)
      setIsLoadingOlder(false)
      return
    }
    setHasError(false)
    if (result.data.length === 0) {
      setHasMoreOlder(false)
      setIsLoadingOlder(false)
      return
    }
    setPrependedHistory((prev) => mergeOlderHistory(prev, result.data, liveMessages))
    setIsLoadingOlder(false)
  }, [apiBase, authToken, hasMoreOlder, isLoadingOlder, liveMessages, pageSize, prependedHistory])

  return {
    combinedMessages,
    loadOlder,
    isLoadingOlder,
    hasMoreOlder,
    hasError
  }
}
