import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'

// Must mirror applications/bpmux/src/session/pins.rs's PINS_UPDATE_MARKER.
export const PINS_UPDATE_MARKER = '__BPMUX_PINS_UPDATE__'

export interface Pin {
  message_key: string
  from: string
  ts: string
  text: string
  pinned_by: string
  pinned_at: string
}

// Derive the canonical channel key for a recipient set. Matches the
// Rust server's session::pins::channel_key (which also keys the pins
// storage). Broadcast = null / empty / ['all']. DM = single-name.
// Group = sorted-joined.
export function channelKey(recipient: string[] | null): string {
  if (!recipient || recipient.length === 0) return 'broadcast'
  if (recipient.includes('all')) return 'broadcast'
  return [...recipient].sort().join('+')
}

// Stable per-message pin key: `{ts}|{from}` — same shape the server
// stores, same shape send/receive messages carry. Message IDs are
// renderer-local (`hist-...`, `wsl-...`) so NOT stable across peers.
export function messageKey(m: { from: string; ts: string }): string {
  return `${m.ts}|${m.from}`
}

interface UsePinsArgs {
  apiBase: string | undefined
  authToken: string | undefined
  channel: string
  // Forwarded WS message stream so the hook can refetch on
  // PINS_UPDATE_MARKER (AC7: live peer updates) without polling.
  messages: ChatMessage[]
}

export interface UsePinsResult {
  pins: Pin[]
  pinnedKeys: ReadonlySet<string>
  pin: (m: { from: string; ts: string; text: string }, pinnedBy: string) => Promise<void>
  unpin: (messageKey: string) => Promise<void>
  refetch: () => Promise<void>
}

export function usePins({ apiBase, authToken, channel, messages }: UsePinsArgs): UsePinsResult {
  const [pins, setPins] = useState<Pin[]>([])
  const seenMessageCountRef = useRef<number>(messages.length)

  const fetchPins = useCallback(async (): Promise<void> => {
    if (!apiBase) {
      setPins([])
      return
    }
    try {
      const res = await fetch(
        `${apiBase}/api/pins?channel=${encodeURIComponent(channel)}`,
        { headers: { Authorization: `Bearer ${authToken ?? ''}` } }
      )
      if (!res.ok) {
        setPins([])
        return
      }
      const json = (await res.json()) as Pin[]
      setPins(Array.isArray(json) ? json : [])
    } catch {
      setPins([])
    }
  }, [apiBase, authToken, channel])

  // Refetch on channel change.
  useEffect(() => {
    void fetchPins()
  }, [fetchPins])

  // Subscribe to PINS_UPDATE_MARKER (and BOARD_INVALIDATE_MARKER as a
  // defensive double-tap since some pin flows also mutate the board).
  useEffect(() => {
    const prev = seenMessageCountRef.current
    if (messages.length <= prev) {
      seenMessageCountRef.current = messages.length
      return
    }
    const fresh = messages.slice(prev)
    seenMessageCountRef.current = messages.length
    if (
      fresh.some(
        (m) => m.text === PINS_UPDATE_MARKER || m.text === BOARD_INVALIDATE_MARKER
      )
    ) {
      void fetchPins()
    }
  }, [messages, fetchPins])

  const pin = useCallback(
    async (m: { from: string; ts: string; text: string }, pinnedBy: string): Promise<void> => {
      if (!apiBase) return
      const key = messageKey(m)
      // Optimistic: local state updates immediately; the invalidate
      // broadcast on success will refetch authoritatively. Failure
      // rolls back via the catch below.
      const optimistic: Pin = {
        message_key: key,
        from: m.from,
        ts: m.ts,
        text: m.text,
        pinned_by: pinnedBy,
        pinned_at: new Date().toISOString()
      }
      setPins((prev) => [...prev.filter((p) => p.message_key !== key), optimistic])
      try {
        const res = await fetch(`${apiBase}/api/pins`, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${authToken ?? ''}`,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({
            channel,
            message_key: key,
            from: m.from,
            ts: m.ts,
            text: m.text,
            pinned_by: pinnedBy
          })
        })
        if (!res.ok) {
          setPins((prev) => prev.filter((p) => p.message_key !== key))
        }
      } catch {
        setPins((prev) => prev.filter((p) => p.message_key !== key))
      }
    },
    [apiBase, authToken, channel]
  )

  const unpin = useCallback(
    async (key: string): Promise<void> => {
      if (!apiBase) return
      const before = pins
      setPins((prev) => prev.filter((p) => p.message_key !== key))
      try {
        const res = await fetch(`${apiBase}/api/pins`, {
          method: 'DELETE',
          headers: {
            Authorization: `Bearer ${authToken ?? ''}`,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ channel, message_key: key })
        })
        if (!res.ok) {
          setPins(before)
        }
      } catch {
        setPins(before)
      }
    },
    [apiBase, authToken, channel, pins]
  )

  const pinnedKeys = useMemo(() => new Set(pins.map((p) => p.message_key)), [pins])
  return { pins, pinnedKeys, pin, unpin, refetch: fetchPins }
}
