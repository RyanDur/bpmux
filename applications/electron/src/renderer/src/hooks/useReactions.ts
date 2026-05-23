import { useCallback } from 'react'
import type { ReactionAggregateWire } from '../transport/types'

export interface AggregatedReaction {
  message_key: string
  emoji: string
  count: number
  users: string[]
}

interface UseReactionsArgs {
  apiBase: string | undefined
  authToken: string | undefined
  channel: string
  mirrorReactions?: ReadonlyMap<string, ReactionAggregateWire[]>
}

export interface UseReactionsResult {
  byMessage: Map<string, AggregatedReaction[]>
  toggle: (messageKey: string, emoji: string, user: string) => Promise<void>
}

export const useReactions = ({
  apiBase,
  authToken,
  channel,
  mirrorReactions
}: UseReactionsArgs): UseReactionsResult => {
  const fromMirror = mirrorReactions?.get(channel) ?? []

  const toggle = useCallback(
    async (messageKey: string, emoji: string, user: string): Promise<void> => {
      if (!apiBase) return
      try {
        await fetch(`${apiBase}/api/reactions`, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${authToken ?? ''}`,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ channel, message_key: messageKey, emoji, user })
        })
      } catch (_e) {
        void _e
      }
    },
    [apiBase, authToken, channel]
  )

  const byMessage = new Map<string, AggregatedReaction[]>()
  for (const r of fromMirror) {
    const list = byMessage.get(r.message_key)
    if (list) list.push(r)
    else byMessage.set(r.message_key, [r])
  }

  return { byMessage, toggle }
}
