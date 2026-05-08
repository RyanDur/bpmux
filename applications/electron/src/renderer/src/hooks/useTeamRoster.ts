import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ChatMessage, RosterMemberWire } from '../transport/types'

export interface RosterEntry {
  identity: string
  role: string
}

export const wireToRosterEntry = (wire: RosterMemberWire): RosterEntry => ({
  identity: wire.identity,
  role: wire.name
})

export const selectRoster = (
  mirrorRoster: readonly RosterMemberWire[] | undefined,
  restRoster: readonly RosterEntry[]
): readonly RosterEntry[] =>
  mirrorRoster && mirrorRoster.length > 0 ? mirrorRoster.map(wireToRosterEntry) : restRoster

interface Props {
  apiBase: string | undefined
  authToken: string | undefined
  messages: ChatMessage[]
  mirrorRoster?: readonly RosterMemberWire[]
}

export function useTeamRoster({
  apiBase,
  authToken,
  messages,
  mirrorRoster
}: Props): readonly RosterEntry[] {
  void messages
  const [restRoster, setRestRoster] = useState<readonly RosterEntry[]>([])
  const roster = useMemo(
    () => selectRoster(mirrorRoster, restRoster),
    [mirrorRoster, restRoster]
  )

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
        if (Array.isArray(json.roster)) setRestRoster(json.roster)
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
    void fetchOnce()
    return () => {
      cancelled = true
      controller.abort()
    }
  }, [apiBase, buildFetch])

  return roster
}
