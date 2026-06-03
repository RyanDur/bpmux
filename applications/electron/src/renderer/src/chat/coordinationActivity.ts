import type { CoordinationPair } from './coordinationPairs'

export const RECENCY_WINDOW_MS = 5 * 60 * 1000

export const relativeTime = (ts: string, nowMs: number): string => {
  const diff = Math.max(0, nowMs - Date.parse(ts))
  const sec = Math.floor(diff / 1000)
  if (sec < 60) return 'now'
  const min = Math.floor(sec / 60)
  if (min < 60) return `${min}m`
  const hr = Math.floor(min / 60)
  if (hr < 24) return `${hr}h`
  return `${Math.floor(hr / 24)}d`
}

export interface GroupActivity {
  lastTs: string
  count: number
  recent: boolean
}

export interface AnnotatedGroup {
  names: readonly string[]
  activity: GroupActivity | null
}

const pairKey = (names: readonly string[]): string => [...names].sort().join('|')

export const orderGroupsByActivity = (
  groups: readonly (readonly string[])[],
  pairs: readonly CoordinationPair[],
  nowMs: number
): readonly AnnotatedGroup[] => {
  const byKey = new Map<string, CoordinationPair>()
  for (const p of pairs) byKey.set(pairKey([p.agentA, p.agentB]), p)
  const annotated: AnnotatedGroup[] = groups.map((names) => {
    const match = names.length === 2 ? byKey.get(pairKey(names)) : undefined
    const activity: GroupActivity | null = match
      ? {
          lastTs: match.lastTs,
          count: match.count,
          recent: nowMs - Date.parse(match.lastTs) <= RECENCY_WINDOW_MS
        }
      : null
    return { names, activity }
  })
  return annotated
    .map((group, index) => ({ group, index }))
    .sort((x, y) => {
      const ax = x.group.activity
      const ay = y.group.activity
      const xr = ax?.recent === true
      const yr = ay?.recent === true
      if (xr && yr && ax && ay) return ay.lastTs.localeCompare(ax.lastTs) || x.index - y.index
      if (xr) return -1
      if (yr) return 1
      return x.index - y.index
    })
    .map(({ group }) => group)
}
