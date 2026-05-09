import { useCallback, useEffect, useState } from 'react'

const storageKey = (viewerIdentity: string): string =>
  `bpmux-epic-collapse:${viewerIdentity}`

const readStored = (viewerIdentity: string): Record<string, boolean> => {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(storageKey(viewerIdentity))
    if (!raw) return {}
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return {}
    const out: Record<string, boolean> = {}
    for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof v === 'boolean') out[k] = v
    }
    return out
  } catch {
    return {}
  }
}

const writeStored = (viewerIdentity: string, state: Record<string, boolean>): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(viewerIdentity), JSON.stringify(state))
  } catch {
    return
  }
}

export interface EpicCollapseStorage {
  isOpen: (epicNumber: number, defaultOpen: boolean) => boolean
  setOpen: (epicNumber: number, open: boolean) => void
}

export const useEpicCollapse = (viewerIdentity: string): EpicCollapseStorage => {
  const [stored, setStored] = useState<Record<string, boolean>>(() =>
    readStored(viewerIdentity)
  )
  useEffect(() => {
    writeStored(viewerIdentity, stored)
  }, [viewerIdentity, stored])
  const isOpen = useCallback(
    (epicNumber: number, defaultOpen: boolean): boolean => {
      const key = String(epicNumber)
      return key in stored ? stored[key] : defaultOpen
    },
    [stored]
  )
  const setOpen = useCallback((epicNumber: number, open: boolean): void => {
    setStored((prev) => ({ ...prev, [String(epicNumber)]: open }))
  }, [])
  return { isOpen, setOpen }
}
