import { useCallback, useMemo, useRef, useState } from 'react'
import type { ChatMessage } from '../transport/types'

// #1153: paged-history hook. Layered on top of `useWebSocket`'s
// bounded live messages array (slice 2 — capped at MAX_MESSAGES via
// drop-tail-on-push). When the user scrolls up to read older history,
// the caller fires `loadOlder()`; this hook fetches older messages
// from the server's paged-history endpoint
// (slice 1's `/api/messages?before=<rfc3339>&limit=N`) and prepends
// them to a separate `prependedHistory` buffer. The exposed
// `combinedMessages` is `[...prependedHistory, ...liveMessages]` in
// chronological order — drop-in replacement for callers that today
// consume `useWebSocket().messages`.
//
// Memory shape: liveMessages is bounded by useWebSocket's cap (drop-
// tail on every push). prependedHistory grows only when the user
// asks for older history via loadOlder; user-controlled growth that
// stops when user reaches the start of available history.
//
// UX awareness (preserve-scroll-when-reading-older / pinned-message
// exemption) layers in the consuming component (slice 4); the hook's
// contract is just the loadOlder action + the combined view.

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
  /// Live messages from `useWebSocket`. The hook consumes this as
  /// the ground-truth "live" stream and combines with its prepended-
  /// history buffer.
  liveMessages: ChatMessage[]
  /// Backend base URL for the paged-history fetch. When omitted,
  /// loadOlder is a no-op (offline / no-backend mode).
  apiBase?: string
  /// Bearer token for the paged-history fetch. When omitted with an
  /// apiBase set, loadOlder still fires unauthenticated (matches
  /// existing initial-load behavior — server's auth is loopback).
  authToken?: string
  /// How many messages to fetch per loadOlder call. Default 200.
  pageSize?: number
}

export interface UseMessageHistoryResult {
  /// Combined view: prepended-history-then-live in chronological
  /// order. Drop-in for `useWebSocket().messages` callers.
  combinedMessages: ChatMessage[]
  /// Fetch the next page of older history from the server. Idempotent
  /// when already-loading or when no-more-older is known. Resolves
  /// silently on the no-op path.
  loadOlder: () => Promise<void>
  /// True while a loadOlder fetch is in flight. Caller can dim the
  /// scroll-up affordance or show a loading-pill.
  isLoadingOlder: boolean
  /// False when the most-recent loadOlder fetch returned 0 messages
  /// (i.e., we've reached the start of available history). Caller
  /// can hide the "load older" affordance.
  hasMoreOlder: boolean
}

function messageKey(m: { from: string; text: string; ts: string }): string {
  return `${m.ts}|${m.from}|${m.text}`
}

// #1153: pure helper that merges newly-fetched older rows into the
// existing prepended-history buffer. Dedupes by message-key against
// both the existing prepended buffer AND the live messages so a
// boundary-overlap (server's strict-less-than vs client's view) or
// a re-fetch-after-WS-arrival doesn't double-render the same row.
// Returns the new prepended-buffer with newly-fetched rows
// prepended in chronological order.
//
// Pure function to enable testability without renderHook
// infrastructure.
export function mergeOlderHistory(
  prependedHistory: ChatMessage[],
  olderRows: HistoryMessage[],
  liveMessages: ChatMessage[]
): ChatMessage[] {
  // Server returns newest-first per existing /api/messages ordering;
  // sort to chronological for prepend.
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

export function useMessageHistory({
  liveMessages,
  apiBase,
  authToken,
  pageSize = DEFAULT_PAGE_SIZE
}: UseMessageHistoryOptions): UseMessageHistoryResult {
  const [prependedHistory, setPrependedHistory] = useState<ChatMessage[]>([])
  const [isLoadingOlder, setIsLoadingOlder] = useState(false)
  const [hasMoreOlder, setHasMoreOlder] = useState(true)
  const inFlightRef = useRef(false)

  const combinedMessages = useMemo(
    () => [...prependedHistory, ...liveMessages],
    [prependedHistory, liveMessages]
  )

  const loadOlder = useCallback(async (): Promise<void> => {
    if (!apiBase) return
    if (inFlightRef.current) return
    if (!hasMoreOlder) return

    // Find the oldest currently-rendered timestamp to use as the
    // `before` boundary. If both prepended and live are empty, no
    // boundary is known — server returns most-recent N which is
    // identical to initial-load and not useful here.
    const oldest =
      prependedHistory[0]?.ts ?? liveMessages[0]?.ts ?? null
    if (!oldest) return

    inFlightRef.current = true
    setIsLoadingOlder(true)
    try {
      const url = `${apiBase}/api/messages?chat_only=true&before=${encodeURIComponent(oldest)}&limit=${pageSize}`
      const headers: Record<string, string> = authToken
        ? { Authorization: `Bearer ${authToken}` }
        : {}
      const res = await fetch(url, { headers })
      if (!res.ok) {
        throw new Error(`paged history fetch: ${res.status}`)
      }
      const page = (await res.json()) as HistoryMessage[]
      if (page.length === 0) {
        setHasMoreOlder(false)
        return
      }
      setPrependedHistory((prev) => mergeOlderHistory(prev, page, liveMessages))
    } catch (err) {
      console.error('bpmux: loadOlder failed', err)
    } finally {
      inFlightRef.current = false
      setIsLoadingOlder(false)
    }
  }, [apiBase, authToken, hasMoreOlder, liveMessages, pageSize, prependedHistory])

  return {
    combinedMessages,
    loadOlder,
    isLoadingOlder,
    hasMoreOlder
  }
}
