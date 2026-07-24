import { useCallback, useEffect, useMemo, useState } from 'react'

const storageKey = (prefix: string, viewerIdentity: string): string => `${prefix}:${viewerIdentity}`

const readStored = (prefix: string, viewerIdentity: string): Record<string, boolean> => {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(storageKey(prefix, viewerIdentity))
    if (!raw) return {}
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return {}
    const out: Record<string, boolean> = {}
    for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof value === 'boolean') out[key] = value
    }
    return out
  } catch {
    return {}
  }
}

const writeStored = (
  prefix: string,
  viewerIdentity: string,
  state: Record<string, boolean>
): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(prefix, viewerIdentity), JSON.stringify(state))
  } catch {
    return
  }
}

export type CollapseMap = {
  isOpen: (key: string, defaultOpen: boolean) => boolean
  setOpen: (key: string, open: boolean) => void
}

export const useCollapseMap = (storagePrefix: string, viewerIdentity: string): CollapseMap => {
  const [stored, setStored] = useState<Record<string, boolean>>(() =>
    readStored(storagePrefix, viewerIdentity)
  )
  useEffect(() => {
    writeStored(storagePrefix, viewerIdentity, stored)
  }, [storagePrefix, viewerIdentity, stored])
  const isOpen = useCallback(
    (key: string, defaultOpen: boolean): boolean => (key in stored ? stored[key] : defaultOpen),
    [stored]
  )
  const setOpen = useCallback((key: string, open: boolean): void => {
    setStored((prev) => ({ ...prev, [key]: open }))
  }, [])
  return useMemo(() => ({ isOpen, setOpen }), [isOpen, setOpen])
}
