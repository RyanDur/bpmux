import { useCallback } from 'react'
import { useCollapseMap } from './useCollapseMap'

export interface EpicCollapseStorage {
  isOpen: (epicId: number, defaultOpen: boolean) => boolean
  setOpen: (epicId: number, open: boolean) => void
}

export const useEpicCollapse = (viewerIdentity: string): EpicCollapseStorage => {
  const map = useCollapseMap('bpmux-epic-collapse', viewerIdentity)
  const isOpen = useCallback(
    (epicId: number, defaultOpen: boolean): boolean => map.isOpen(String(epicId), defaultOpen),
    [map]
  )
  const setOpen = useCallback(
    (epicId: number, open: boolean): void => map.setOpen(String(epicId), open),
    [map]
  )
  return { isOpen, setOpen }
}
