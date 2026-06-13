import type { ChatMessage } from '../transport/types'

type RosterIdentity = {
  identity: string
  role: string
}

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

export const deriveTeamRoles = (
  messages: readonly ChatMessage[],
  viewer?: string
): readonly string[] => {
  const set = new Set<string>()
  if (viewer !== undefined && viewer.length > 0) set.add(shortName(viewer))
  for (const m of messages) {
    if (m.from && m.from.length > 0) set.add(shortName(m.from))
    if (m.to) {
      for (const t of m.to) {
        if (t !== 'all' && t.length > 0) set.add(t)
      }
    }
  }
  return Array.from(set).sort()
}

export const combinations = (
  roles: readonly string[],
  minSize: number,
  maxSize: number
): string[][] => {
  const sorted = [...roles].sort()
  const out: string[][] = []
  const n = sorted.length
  const clampedMin = Math.max(1, minSize)
  const clampedMax = Math.min(n, maxSize)
  if (clampedMin > clampedMax) return out
  for (let size = clampedMin; size <= clampedMax; size++) {
    const acc: string[] = []
    const walk = (start: number): void => {
      if (acc.length === size) {
        out.push([...acc])
        return
      }
      const remaining = size - acc.length
      for (let i = start; i <= n - remaining; i++) {
        acc.push(sorted[i])
        walk(i + 1)
        acc.pop()
      }
    }
    walk(0)
  }
  return out
}

export const deriveCombinatorialGroups = (roles: readonly string[]): string[][] => {
  if (roles.length < 2) return []
  return combinations(roles, 2, roles.length)
}

export const deriveTeamRolesFromRoster = (
  roster: readonly RosterIdentity[]
): readonly string[] => {
  const set = new Set<string>()
  for (const entry of roster) set.add(entry.role)
  return Array.from(set).sort()
}

const toolSuffixOf = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? '' : identity.slice(idx + 1)
}

const isAutoIdentity = (role: string): boolean => role.startsWith('agent-')

export const deriveDirectRolesFromRoster = (
  roster: readonly RosterIdentity[],
  viewer?: string
): readonly string[] => {
  const self = viewer !== undefined ? shortName(viewer) : null
  const namedTools = new Set<string>()
  for (const e of roster) {
    if (!isAutoIdentity(e.role)) namedTools.add(toolSuffixOf(e.identity))
  }
  const set = new Set<string>()
  for (const e of roster) {
    if (e.role === self) continue
    if (isAutoIdentity(e.role) && namedTools.has(toolSuffixOf(e.identity))) continue
    set.add(e.role)
  }
  return Array.from(set).sort()
}

export const deriveCombinableRolesFromRoster = (
  roster: readonly RosterIdentity[],
  viewer?: string
): readonly string[] => deriveDirectRolesFromRoster(roster, viewer)
