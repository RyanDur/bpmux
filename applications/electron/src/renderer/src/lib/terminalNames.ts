export const resolveTerminalName = (custom: string | undefined, dir: string): string =>
  custom !== undefined && custom.length > 0 ? custom : dir

const storageKey = (identity: string): string => `bpmux-terminal-names:${identity}`

export const readTerminalNames = (identity: string): Record<string, string> => {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(storageKey(identity))
    if (!raw) return {}
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') return {}
    const names: Record<string, string> = {}
    for (const [id, name] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof name === 'string' && name.length > 0) names[id] = name
    }
    return names
  } catch {
    return {}
  }
}

export const writeTerminalNames = (identity: string, names: Record<string, string>): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(identity), JSON.stringify(names))
  } catch {
    return
  }
}
