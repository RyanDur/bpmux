import { useCallback } from 'react'
import { toggleReaction, useReactionsByChannel } from '../transport/session'

export interface AggregatedReaction {
  message_key: string
  emoji: string
  count: number
  users: string[]
}

interface UseReactionsArgs {
  channel: string
}

export interface UseReactionsResult {
  byMessage: Map<string, AggregatedReaction[]>
  toggle: (messageKey: string, emoji: string, user: string) => void
}

export const useReactions = ({ channel }: UseReactionsArgs): UseReactionsResult => {
  const fromMirror = useReactionsByChannel().get(channel) ?? []

  const toggle = useCallback(
    (messageKey: string, emoji: string, user: string): void => {
      toggleReaction(channel, messageKey, emoji, user)
    },
    [channel]
  )

  const byMessage = new Map<string, AggregatedReaction[]>()
  for (const r of fromMirror) {
    const list = byMessage.get(r.message_key)
    if (list) list.push(r)
    else byMessage.set(r.message_key, [r])
  }

  return { byMessage, toggle }
}
