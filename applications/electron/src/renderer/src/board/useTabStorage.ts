import { useCallback, useEffect, useState } from 'react'

const storageKey = (prefix: string, viewerIdentity: string): string => `${prefix}:${viewerIdentity}`

const readActive = <TabId extends string>(
  prefix: string,
  viewerIdentity: string,
  validIds: ReadonlyArray<TabId>,
  defaultId: TabId
): TabId => {
  if (typeof window === 'undefined') return defaultId
  try {
    const raw = window.localStorage.getItem(storageKey(prefix, viewerIdentity))
    if (raw === null) return defaultId
    return validIds.includes(raw as TabId) ? (raw as TabId) : defaultId
  } catch {
    return defaultId
  }
}

const writeActive = (prefix: string, viewerIdentity: string, id: string): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(storageKey(prefix, viewerIdentity), id)
  } catch {
    return
  }
}

export type TabStorage<TabId extends string> = {
  activeId: TabId
  setActiveId: (next: TabId) => void
}

export const useTabStorage = <TabId extends string>(
  storagePrefix: string,
  viewerIdentity: string,
  validIds: ReadonlyArray<TabId>,
  defaultId: TabId
): TabStorage<TabId> => {
  const [activeId, setActiveIdState] = useState<TabId>(() =>
    readActive(storagePrefix, viewerIdentity, validIds, defaultId)
  )
  useEffect(() => {
    writeActive(storagePrefix, viewerIdentity, activeId)
  }, [storagePrefix, viewerIdentity, activeId])
  const setActiveId = useCallback((next: TabId): void => {
    setActiveIdState(next)
  }, [])
  return { activeId, setActiveId }
}
