import { useCallback, useEffect, useState } from 'react'

const storageKey = (viewerIdentity: string): string =>
  `bpmux-iteration-collapse:${viewerIdentity}`

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

export interface IterationCollapseStorage {
  isOpen: (iterationKey: string, defaultOpen: boolean) => boolean
  setOpen: (iterationKey: string, open: boolean) => void
}

export const useIterationCollapse = (viewerIdentity: string): IterationCollapseStorage => {
  const [stored, setStored] = useState<Record<string, boolean>>(() =>
    readStored(viewerIdentity)
  )
  useEffect(() => {
    writeStored(viewerIdentity, stored)
  }, [viewerIdentity, stored])
  const isOpen = useCallback(
    (iterationKey: string, defaultOpen: boolean): boolean => {
      return iterationKey in stored ? stored[iterationKey] : defaultOpen
    },
    [stored]
  )
  const setOpen = useCallback((iterationKey: string, open: boolean): void => {
    setStored((prev) => ({ ...prev, [iterationKey]: open }))
  }, [])
  return { isOpen, setOpen }
}
