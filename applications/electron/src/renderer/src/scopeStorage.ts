import { RENDERER_IDENTITY } from './state'

export type PersistedScope =
  | { kind: 'all' }
  | { kind: 'broadcast' }
  | { kind: 'scoped'; value: string[] }

const SCOPE_STORAGE_KEY = `bpmux-scope:${RENDERER_IDENTITY}`

export const readScope = (): PersistedScope | null => {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(SCOPE_STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') return null
    const obj = parsed as Record<string, unknown>
    if (obj.kind === 'all') return { kind: 'all' }
    if (obj.kind === 'broadcast') return { kind: 'broadcast' }
    if (obj.kind === 'scoped' && Array.isArray(obj.value)) {
      const value = obj.value.filter((v): v is string => typeof v === 'string')
      if (value.length > 0) return { kind: 'scoped', value }
    }
    return null
  } catch {
    return null
  }
}

export const writeScope = (scope: PersistedScope): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(SCOPE_STORAGE_KEY, JSON.stringify(scope))
  } catch {
    return
  }
}
