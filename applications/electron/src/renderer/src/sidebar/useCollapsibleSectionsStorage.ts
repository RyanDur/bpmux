import { useEffect, useState } from 'react'

const STORAGE_PREFIX = 'bpmux-sidebar-collapse:'

const storageKey = (viewerIdentity: string): string => `${STORAGE_PREFIX}${viewerIdentity}`

const readState = <K extends string>(
  viewerIdentity: string,
  defaults: Record<K, boolean>
): Record<K, boolean> => {
  if (typeof window === 'undefined') return { ...defaults }
  try {
    const raw = window.localStorage.getItem(storageKey(viewerIdentity))
    if (!raw) return { ...defaults }
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return { ...defaults }
    const record = parsed as Record<string, unknown>
    const merged: Record<string, boolean> = { ...defaults }
    for (const key of Object.keys(defaults)) {
      const value = record[key]
      if (typeof value === 'boolean') {
        merged[key] = value
      }
    }
    return merged as Record<K, boolean>
  } catch {
    return { ...defaults }
  }
}

const writeState = (viewerIdentity: string, state: Record<string, boolean>): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(viewerIdentity), JSON.stringify(state))
  } catch {
    return
  }
}

export interface CollapsibleSectionsStorage<K extends string> {
  state: Record<K, boolean>
  setState: (next: Record<K, boolean>) => void
}

export const useCollapsibleSectionsStorage = <K extends string>(
  viewerIdentity: string,
  defaults: Record<K, boolean>
): CollapsibleSectionsStorage<K> => {
  const [state, setState] = useState<Record<K, boolean>>(() => readState(viewerIdentity, defaults))
  useEffect(() => {
    writeState(viewerIdentity, state)
  }, [viewerIdentity, state])
  return { state, setState }
}
