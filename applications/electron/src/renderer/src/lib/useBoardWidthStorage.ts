import { useEffect, useState } from 'react'

const storageKey = (prefix: string, viewerIdentity: string): string => `${prefix}:${viewerIdentity}`

const readWidth = (prefix: string, viewerIdentity: string, defaultWidth: number): number => {
  if (typeof window === 'undefined') return defaultWidth
  try {
    const raw = window.localStorage.getItem(storageKey(prefix, viewerIdentity))
    if (!raw) return defaultWidth
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed === 'number' && Number.isFinite(parsed)) return parsed
    return defaultWidth
  } catch {
    return defaultWidth
  }
}

const writeWidth = (prefix: string, viewerIdentity: string, width: number): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(prefix, viewerIdentity), JSON.stringify(width))
  } catch {
    return
  }
}

export interface BoardWidthStorage {
  width: number
  setWidth: (next: number) => void
}

export const useBoardWidthStorage = (
  storagePrefix: string,
  viewerIdentity: string,
  defaultWidth: number
): BoardWidthStorage => {
  const [width, setWidth] = useState<number>(() =>
    readWidth(storagePrefix, viewerIdentity, defaultWidth)
  )
  useEffect(() => {
    writeWidth(storagePrefix, viewerIdentity, width)
  }, [storagePrefix, viewerIdentity, width])
  return { width, setWidth }
}
