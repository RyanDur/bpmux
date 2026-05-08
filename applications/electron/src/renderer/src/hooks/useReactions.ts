import { useCallback, useEffect, useState } from 'react'
import type { ReactionAggregateWire } from '../transport/types'

export interface AggregatedReaction {
  message_key: string
  emoji: string
  count: number
  users: string[]
}

export const selectReactions = (
  mirrorEntry: readonly ReactionAggregateWire[] | undefined,
  restAggregated: readonly AggregatedReaction[]
): readonly AggregatedReaction[] => mirrorEntry ?? restAggregated

interface UseReactionsArgs {
  apiBase: string | undefined
  authToken: string | undefined
  channel: string
  mirrorReactions?: ReadonlyMap<string, ReactionAggregateWire[]>
}

export interface UseReactionsResult {
  // Keyed by message_key for O(1) lookup per row render.
  byMessage: Map<string, AggregatedReaction[]>
  toggle: (messageKey: string, emoji: string, user: string) => Promise<void>
  refetch: () => Promise<void>
}

export function useReactions({
  apiBase,
  authToken,
  channel,
  mirrorReactions
}: UseReactionsArgs): UseReactionsResult {
  const [aggregated, setAggregated] = useState<AggregatedReaction[]>([])
  const effectiveAggregated = selectReactions(mirrorReactions?.get(channel), aggregated)

  const fetchReactions = useCallback(async (): Promise<void> => {
    if (!apiBase) {
      setAggregated([])
      return
    }
    try {
      const res = await fetch(
        `${apiBase}/api/reactions?channel=${encodeURIComponent(channel)}`,
        { headers: { Authorization: `Bearer ${authToken ?? ''}` } }
      )
      if (!res.ok) {
        setAggregated([])
        return
      }
      const json = (await res.json()) as AggregatedReaction[]
      setAggregated(Array.isArray(json) ? json : [])
    } catch {
      setAggregated([])
    }
  }, [apiBase, authToken, channel])

  useEffect(() => {
    void fetchReactions()
  }, [fetchReactions])

  const toggle = useCallback(
    async (messageKey: string, emoji: string, user: string): Promise<void> => {
      if (!apiBase) return
      setAggregated((prev) => {
        const idx = prev.findIndex(
          (r) => r.message_key === messageKey && r.emoji === emoji
        )
        if (idx === -1) {
          return [
            ...prev,
            { message_key: messageKey, emoji, count: 1, users: [user] }
          ]
        }
        const r = prev[idx]
        const userIdx = r.users.indexOf(user)
        const nextUsers =
          userIdx === -1 ? [...r.users, user] : r.users.filter((u) => u !== user)
        if (nextUsers.length === 0) {
          return prev.filter((_, i) => i !== idx)
        }
        return prev.map((x, i) =>
          i === idx ? { ...r, users: nextUsers, count: nextUsers.length } : x
        )
      })
      try {
        const res = await fetch(`${apiBase}/api/reactions`, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${authToken ?? ''}`,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ channel, message_key: messageKey, emoji, user })
        })
        if (!res.ok) {
          // Rollback by refetching the authoritative state.
          void fetchReactions()
        }
      } catch {
        void fetchReactions()
      }
    },
    [apiBase, authToken, channel, fetchReactions]
  )

  const byMessage = new Map<string, AggregatedReaction[]>()
  for (const r of effectiveAggregated) {
    const list = byMessage.get(r.message_key)
    if (list) list.push(r)
    else byMessage.set(r.message_key, [r])
  }

  return { byMessage, toggle, refetch: fetchReactions }
}
