import { useCallback, useEffect, useState } from 'react'

// The four classes bpmux emits today. Pills render in this order.
export const MESSAGE_CLASSES = ['chat', 'insight', 'activity', 'system'] as const
export type MessageClass = (typeof MESSAGE_CLASSES)[number]

const STORAGE_KEY = 'bpmux-message-filter'

// Default: chat + insight visible. Activity + system were hardcoded-hidden
// in phase-1a (noise that buried signal); the filter lets the user opt
// them back in when they want the full firehose.
const DEFAULT_VISIBLE: readonly MessageClass[] = ['chat', 'insight']

const readStored = (): MessageClass[] | null => {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return null
    const valid = parsed.filter(
      (v): v is MessageClass =>
        typeof v === 'string' && (MESSAGE_CLASSES as readonly string[]).includes(v)
    )
    return valid
  } catch {
    return null
  }
}

const resolveInitial = (): Set<MessageClass> => {
  const stored = readStored()
  return new Set(stored ?? DEFAULT_VISIBLE)
}

export interface MessageFilter {
  visible: Set<MessageClass>
  toggle: (cls: MessageClass) => void
  setAll: () => void
  // Restore the default filter (chat + insight). Used when sidebar
  // navigates back to broadcast / DM / group — callers want the
  // normal view, not whatever narrow filter the user last set.
  reset: () => void
  // Narrow the filter to exactly one class (sidebar SYSTEM click).
  setOnly: (cls: MessageClass) => void
  isAllActive: boolean
}

// Classifies a message's messageType into our canonical class set.
// Anything unrecognised (including missing messageType) falls through
// to 'chat' — keeps historical messages without a class tag visible
// under the default filter.
export const classOf = (messageType: string | null | undefined): MessageClass => {
  if (messageType && (MESSAGE_CLASSES as readonly string[]).includes(messageType)) {
    return messageType as MessageClass
  }
  return 'chat'
}

export const useMessageFilter = (): MessageFilter => {
  const [visible, setVisible] = useState<Set<MessageClass>>(resolveInitial)

  useEffect(() => {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify([...visible]))
    } catch {
      // storage can fail silently (private mode, quota); filter still
      // works per-session, just doesn't survive reload.
    }
  }, [visible])

  const toggle = useCallback((cls: MessageClass): void => {
    setVisible((prev) => {
      const next = new Set(prev)
      if (next.has(cls)) next.delete(cls)
      else next.add(cls)
      return next
    })
  }, [])

  const setAll = useCallback((): void => {
    setVisible(new Set(MESSAGE_CLASSES))
  }, [])

  const reset = useCallback((): void => {
    setVisible(new Set(DEFAULT_VISIBLE))
  }, [])

  const setOnly = useCallback((cls: MessageClass): void => {
    setVisible(new Set([cls]))
  }, [])

  const isAllActive = visible.size === MESSAGE_CLASSES.length

  return { visible, toggle, setAll, reset, setOnly, isAllActive }
}
