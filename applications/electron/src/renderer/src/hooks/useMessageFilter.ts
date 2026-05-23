import { useCallback, useEffect, useMemo, useState } from 'react'

export const MESSAGE_CLASSES = ['chat', 'events', 'bpmux'] as const
export type MessageClass = (typeof MESSAGE_CLASSES)[number]

const STORAGE_PREFIX = 'bpmux-message-filter'

const DEFAULT_VISIBLE: readonly MessageClass[] = ['chat']

const storageKey = (channelKey: string): string => `${STORAGE_PREFIX}:${channelKey}`

const isMessageClass = (v: unknown): v is MessageClass =>
  typeof v === 'string' && (MESSAGE_CLASSES as readonly string[]).includes(v)

const readStored = (channelKey: string): MessageClass[] | null => {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(storageKey(channelKey))
    if (raw === null) return null
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return null
    return parsed.filter(isMessageClass)
  } catch {
    return null
  }
}

const resolveInitial = (channelKey: string): Set<MessageClass> => {
  const stored = readStored(channelKey)
  return new Set(stored ?? DEFAULT_VISIBLE)
}

export interface MessageFilter {
  visible: Set<MessageClass>
  toggle: (cls: MessageClass) => void
  reset: () => void
  setOnly: (cls: MessageClass) => void
}

const LEGACY_TYPE_REMAP: Record<string, MessageClass> = {
  system: 'bpmux',
  activity: 'events',
  insight: 'events',
  'state-delta': 'events',
  'state-snapshot': 'events',
  nudge: 'events'
}

export const classOf = (messageType: string | null | undefined): MessageClass => {
  if (!messageType) return 'chat'
  if (isMessageClass(messageType)) return messageType
  const remapped = LEGACY_TYPE_REMAP[messageType]
  if (remapped !== undefined) return remapped
  return 'chat'
}

export const useMessageFilter = (channelKey: string): MessageFilter => {
  const [byChannel, setByChannel] = useState<Map<string, Set<MessageClass>>>(() => new Map())

  const visible = useMemo<Set<MessageClass>>(
    () => byChannel.get(channelKey) ?? resolveInitial(channelKey),
    [byChannel, channelKey]
  )

  useEffect(() => {
    try {
      window.localStorage.setItem(storageKey(channelKey), JSON.stringify([...visible]))
    } catch {
      return
    }
  }, [channelKey, visible])

  const updateChannel = useCallback(
    (key: string, mutator: (prev: Set<MessageClass>) => Set<MessageClass>): void => {
      setByChannel((prev) => {
        const current = prev.get(key) ?? resolveInitial(key)
        const next = mutator(current)
        const out = new Map(prev)
        out.set(key, next)
        return out
      })
    },
    []
  )

  const toggle = useCallback(
    (cls: MessageClass): void => {
      updateChannel(channelKey, (prev) => {
        const next = new Set(prev)
        if (next.has(cls)) next.delete(cls)
        else next.add(cls)
        return next
      })
    },
    [channelKey, updateChannel]
  )

  const reset = useCallback((): void => {
    updateChannel(channelKey, () => new Set(DEFAULT_VISIBLE))
  }, [channelKey, updateChannel])

  const setOnly = useCallback(
    (cls: MessageClass): void => {
      updateChannel(channelKey, () => new Set([cls]))
    },
    [channelKey, updateChannel]
  )

  return { visible, toggle, reset, setOnly }
}
