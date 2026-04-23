import { useCallback, useEffect, useRef, useState } from 'react'
import { ROSTER_UPDATE_MARKER, type ChatMessage } from '../transport/types'

// #402 team-roster substrate. Source of truth for sidebar
// membership (directs + combinatorial groups). Previously the
// sidebar derived membership from observed message senders, which
// meant teammates disappeared as the hydration window narrowed
// past their last message. Ryan 2026-04-23 12:08 UTC: "keep
// losing the groups and directs." Fix: fetch the authoritative
// roster from `/api/roster` + subscribe to `ROSTER_UPDATE_MARKER`
// so the set stays in sync with actual bridge connections.

export interface RosterEntry {
  identity: string
  role: string
}

interface Props {
  apiBase: string | undefined
  authToken: string | undefined
  messages: ChatMessage[]
}

/**
 * Fetches the connected-agent roster once at mount, then refetches
 * whenever a `ROSTER_UPDATE_MARKER` arrives on the message stream.
 * Returns the current snapshot. Empty array before the first fetch
 * completes or when the bridge has no peers yet.
 */
export function useTeamRoster({ apiBase, authToken, messages }: Props): readonly RosterEntry[] {
  const [roster, setRoster] = useState<readonly RosterEntry[]>([])
  // Seen-count pattern (mirrors useReactions / useCiGlyph): only
  // react to NEW marker arrivals, not the hydration backfill on
  // mount. Prevents a stampede of refetches when the initial
  // message window resolves.
  const seenMessageCountRef = useRef<number>(messages.length)
  const fetchOnceRef = useRef<() => Promise<void>>(async () => {})

  const buildFetch = useCallback(
    (signal: AbortSignal, cancelledCheck: () => boolean) => async (): Promise<void> => {
      if (!apiBase) return
      try {
        const res = await fetch(`${apiBase}/api/roster`, {
          headers: { Authorization: `Bearer ${authToken ?? ''}` },
          signal
        })
        if (!res.ok) return
        const json = (await res.json()) as { roster?: RosterEntry[] }
        if (cancelledCheck()) return
        if (Array.isArray(json.roster)) setRoster(json.roster)
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
        // Soft-fail: leave previous roster intact. A later marker or
        // the next mount cycle will retry. Sidebar doesn't flash
        // empty on a transient fetch error.
      }
    },
    [apiBase, authToken]
  )

  useEffect(() => {
    if (!apiBase) return
    let cancelled = false
    const controller = new AbortController()
    const fetchOnce = buildFetch(controller.signal, () => cancelled)
    fetchOnceRef.current = fetchOnce
    void fetchOnce()
    return () => {
      cancelled = true
      controller.abort()
    }
  }, [apiBase, buildFetch])

  // Push-based refresh: ROSTER_UPDATE_MARKER arriving in the message
  // stream triggers a refetch. Gated by seenMessageCountRef so
  // hydration backfill doesn't flood refetches.
  useEffect(() => {
    const prev = seenMessageCountRef.current
    seenMessageCountRef.current = messages.length
    if (messages.length <= prev) return
    const newMessages = messages.slice(prev)
    const hasRosterUpdate = newMessages.some((m) => m.text === ROSTER_UPDATE_MARKER)
    if (hasRosterUpdate) void fetchOnceRef.current()
  }, [messages])

  return roster
}
