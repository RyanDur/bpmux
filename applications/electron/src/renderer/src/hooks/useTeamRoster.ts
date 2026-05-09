import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ConnectionStatus, RosterMemberWire } from '../transport/types'
import { useRefetchOnReconnect } from '../board/useRefetchOnReconnect'

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
  mirrorRoster?: readonly RosterMemberWire[]
  wsStatus?: ConnectionStatus
}

export function useTeamRoster({
  apiBase,
  authToken,
  mirrorRoster,
  wsStatus
}: Props): readonly RosterEntry[] {
  const [restRoster, setRestRoster] = useState<readonly RosterEntry[]>([])
  const roster = useMemo(
    () => selectRoster(mirrorRoster, restRoster),
    [mirrorRoster, restRoster]
  )

  const fetchRoster = useCallback(
    async (signal?: AbortSignal): Promise<void> => {
      if (!apiBase) return
      try {
        const res = await fetch(`${apiBase}/api/roster`, {
          headers: { Authorization: `Bearer ${authToken ?? ''}` },
          signal
        })
        if (!res.ok) return
        const json = (await res.json()) as { roster?: RosterEntry[] }
        if (signal?.aborted) return
        if (Array.isArray(json.roster)) setRestRoster(json.roster)
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
      }
    },
    [apiBase, authToken]
  )

  useEffect(() => {
    if (!apiBase) return
    const controller = new AbortController()
    void fetchRoster(controller.signal)
    return () => {
      controller.abort()
    }
  }, [apiBase, fetchRoster])

  useRefetchOnReconnect(wsStatus ?? 'connected', () => {
    void fetchRoster()
  })

  return roster
}
