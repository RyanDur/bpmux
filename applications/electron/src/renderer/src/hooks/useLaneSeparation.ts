import { useCallback, useEffect, useState } from 'react'

// #374 lane separation preference. When on, MessageList groups
// visible messages by class (chat / insight / activity / system)
// and renders each group in its own section with a sticky header
// — letting the reader scan each class independently without
// toggling the filter pills. When off (default), messages render
// interleaved in chronological order (pre-#374 behavior).

export type LaneMode = 'off' | 'on'

const STORAGE_KEY = 'bpmux-lanes'

function resolveInitial(): LaneMode {
  if (typeof window === 'undefined') return 'off'
  const stored = window.localStorage.getItem(STORAGE_KEY)
  if (stored === 'on' || stored === 'off') return stored
  return 'off'
}

export function useLaneSeparation(): {
  lanes: LaneMode
  toggle: () => void
  set: (m: LaneMode) => void
} {
  const [lanes, setLanes] = useState<LaneMode>(resolveInitial)

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.lanes = lanes
    }
    try {
      window.localStorage.setItem(STORAGE_KEY, lanes)
    } catch {
      // private mode / storage disabled — non-load-bearing on
      // correctness; preference simply won't persist this session.
    }
  }, [lanes])

  const toggle = useCallback((): void => {
    setLanes((prev) => (prev === 'off' ? 'on' : 'off'))
  }, [])

  const set = useCallback((m: LaneMode): void => {
    setLanes(m)
  }, [])

  return { lanes, toggle, set }
}
