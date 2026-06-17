import { useCallback } from 'react'
import { useCollapseMap } from './useCollapseMap'

export interface EpicCollapseStorage {
  isOpen: (epicNumber: number, defaultOpen: boolean) => boolean
  setOpen: (epicNumber: number, open: boolean) => void
}

export const useEpicCollapse = (viewerIdentity: string): EpicCollapseStorage => {
  const map = useCollapseMap('bpmux-epic-collapse', viewerIdentity)
  const isOpen = useCallback(
    (epicNumber: number, defaultOpen: boolean): boolean =>
      map.isOpen(String(epicNumber), defaultOpen),
    [map]
  )
  const setOpen = useCallback(
    (epicNumber: number, open: boolean): void => map.setOpen(String(epicNumber), open),
    [map]
  )
  return { isOpen, setOpen }
}
