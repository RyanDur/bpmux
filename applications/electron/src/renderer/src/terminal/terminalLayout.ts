export type TerminalLayoutHint = {
  order: string[]
  activeId: string | null
}

export type RestoredLayout = {
  orderedIds: string[]
  activeId: string | null
}

export const restoreTerminalLayout = (
  liveIds: readonly string[],
  hint: TerminalLayoutHint | null
): RestoredLayout => {
  const liveSet = new Set(liveIds)
  const ordered = (hint?.order ?? []).filter((id) => liveSet.has(id))
  const seen = new Set(ordered)
  for (const id of liveIds) {
    if (!seen.has(id)) {
      ordered.push(id)
      seen.add(id)
    }
  }
  const activeId =
    hint?.activeId != null && liveSet.has(hint.activeId) ? hint.activeId : (ordered[0] ?? null)
  return { orderedIds: ordered, activeId }
}

const storageKey = (identity: string): string => `bpmux-terminals:${identity}`

export const readTerminalLayout = (identity: string): TerminalLayoutHint | null => {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(storageKey(identity))
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') return null
    const obj = parsed as Record<string, unknown>
    const order = Array.isArray(obj.order)
      ? obj.order.filter((v): v is string => typeof v === 'string')
      : []
    const activeId = typeof obj.activeId === 'string' ? obj.activeId : null
    return { order, activeId }
  } catch {
    return null
  }
}

export const writeTerminalLayout = (identity: string, hint: TerminalLayoutHint): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(identity), JSON.stringify(hint))
  } catch {
    return
  }
}
