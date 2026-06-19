import { useCallback, useMemo, useState } from 'react'
import { requestOlderHistory } from '../transport/session'
import { asMessageType } from '../transport/messageType'
import type { ChatMessage, HistoryMessageWire } from '../transport/types'

const DEFAULT_PAGE_SIZE = 200

type HistoryMessage = HistoryMessageWire

export interface UseMessageHistoryOptions {
  liveMessages: ChatMessage[]
  apiBase?: string
  pageSize?: number
}

export interface UseMessageHistoryResult {
  combinedMessages: ChatMessage[]
  loadOlder: () => Promise<void>
  isLoadingOlder: boolean
  hasMoreOlder: boolean
  hasError: boolean
}

const messageKey = (m: { from: string; text: string; ts: string }): string =>
  `${m.ts}|${m.from}|${m.text}`

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
      messageType: asMessageType(h.message_type)
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
    let rows: HistoryMessage[]
    try {
      rows = await requestOlderHistory(oldest, pageSize)
    } catch {
      setHasError(true)
      setIsLoadingOlder(false)
      return
    }
    setHasError(false)
    if (rows.length === 0) {
      setHasMoreOlder(false)
      setIsLoadingOlder(false)
      return
    }
    setPrependedHistory((prev) => mergeOlderHistory(prev, rows, liveMessages))
    setIsLoadingOlder(false)
  }, [apiBase, hasMoreOlder, isLoadingOlder, liveMessages, pageSize, prependedHistory])

  return {
    combinedMessages,
    loadOlder,
    isLoadingOlder,
    hasMoreOlder,
    hasError
  }
}
