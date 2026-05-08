import { useCallback, useEffect, useMemo, useState } from 'react'
import type { PinnedMessageWire } from '../transport/types'

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
  mirrorPins?: ReadonlyMap<string, PinnedMessageWire[]>
}

export const wireToPin = (wire: PinnedMessageWire): Pin => ({
  message_key: wire.message_key,
  from: wire.from,
  ts: wire.ts,
  text: wire.text,
  pinned_by: wire.pinner,
  pinned_at: wire.pinned_at
})

export const selectPins = (
  mirrorEntry: readonly PinnedMessageWire[] | undefined,
  restPins: readonly Pin[]
): readonly Pin[] => (mirrorEntry ? mirrorEntry.map(wireToPin) : restPins)

export interface UsePinsResult {
  pins: Pin[]
  pinnedKeys: ReadonlySet<string>
  pin: (m: { from: string; ts: string; text: string }, pinnedBy: string) => Promise<void>
  unpin: (messageKey: string) => Promise<void>
  refetch: () => Promise<void>
}

export function usePins({
  apiBase,
  authToken,
  channel,
  mirrorPins
}: UsePinsArgs): UsePinsResult {
  const [restPins, setRestPins] = useState<Pin[]>([])
  const pins = useMemo(
    () => [...selectPins(mirrorPins?.get(channel), restPins)],
    [mirrorPins, channel, restPins]
  )

  const fetchPins = useCallback(async (): Promise<void> => {
    if (!apiBase) {
      setRestPins([])
      return
    }
    try {
      const res = await fetch(
        `${apiBase}/api/pins?channel=${encodeURIComponent(channel)}`,
        { headers: { Authorization: `Bearer ${authToken ?? ''}` } }
      )
      if (!res.ok) {
        setRestPins([])
        return
      }
      const json = (await res.json()) as Pin[]
      setRestPins(Array.isArray(json) ? json : [])
    } catch {
      setRestPins([])
    }
  }, [apiBase, authToken, channel])

  // Refetch on channel change.
  useEffect(() => {
    void fetchPins()
  }, [fetchPins])

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
      setRestPins((prev) => [...prev.filter((p) => p.message_key !== key), optimistic])
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
          setRestPins((prev) => prev.filter((p) => p.message_key !== key))
        }
      } catch {
        setRestPins((prev) => prev.filter((p) => p.message_key !== key))
      }
    },
    [apiBase, authToken, channel]
  )

  const unpin = useCallback(
    async (key: string): Promise<void> => {
      if (!apiBase) return
      const before = pins
      setRestPins((prev) => prev.filter((p) => p.message_key !== key))
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
          setRestPins(before)
        }
      } catch {
        setRestPins(before)
      }
    },
    [apiBase, authToken, channel, pins]
  )

  const pinnedKeys = useMemo(() => new Set(pins.map((p) => p.message_key)), [pins])
  return { pins, pinnedKeys, pin, unpin, refetch: fetchPins }
}
