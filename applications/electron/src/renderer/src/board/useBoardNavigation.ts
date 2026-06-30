import { useCallback, useMemo, useRef, useState } from 'react'
import type { Story } from './types'

interface UseBoardNavigationResult {
  asideRef: React.RefObject<HTMLElement | null>
  selectedId: number | null
  setSelectedId: React.Dispatch<React.SetStateAction<number | null>>
  drilledId: number | null
  focusTimelineFor: number | null
  effectiveSelected: number | null
  drilledStory: Story | null
  renderedDrill: Story | null
  handleOpen: (id: number) => void
  handleOpenTimeline: (id: number) => void
  handleBack: () => void
  handleDrillUnmountReady: () => void
  handleKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void
}

export const useBoardNavigation = (
  stories: Story[],
  navOrder: number[]
): UseBoardNavigationResult => {
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [drilledId, setDrilledId] = useState<number | null>(null)
  const [focusTimelineFor, setFocusTimelineFor] = useState<number | null>(null)
  const asideRef = useRef<HTMLElement | null>(null)

  const effectiveSelected =
    selectedId !== null && navOrder.includes(selectedId) ? selectedId : (navOrder[0] ?? null)

  const focusRow = useCallback((id: number | null) => {
    if (id === null) return
    requestAnimationFrame(() => {
      asideRef.current
        ?.querySelector<HTMLElement>(`.board-story-navigate[aria-label^="Open story #${id} "]`)
        ?.focus()
    })
  }, [])

  const drilledStory = useMemo(
    () => (drilledId === null ? null : (stories.find((s) => s.id === drilledId) ?? null)),
    [drilledId, stories]
  )

  const [lastDrilledStory, setLastDrilledStory] = useState<typeof drilledStory>(drilledStory)
  if (drilledStory !== null && drilledStory !== lastDrilledStory) {
    setLastDrilledStory(drilledStory)
  }
  const renderedDrill = drilledStory ?? lastDrilledStory
  const handleDrillUnmountReady = useCallback(() => {
    if (drilledStory === null) setLastDrilledStory(null)
  }, [drilledStory])

  const handleOpen = useCallback((id: number) => {
    setSelectedId(id)
    setDrilledId(id)
    setFocusTimelineFor(null)
  }, [])

  const handleOpenTimeline = useCallback((id: number) => {
    setSelectedId(id)
    setDrilledId(id)
    setFocusTimelineFor(id)
  }, [])

  const handleBack = useCallback(() => {
    setDrilledId(null)
    setFocusTimelineFor(null)
    focusRow(effectiveSelected)
  }, [effectiveSelected, focusRow])

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      if (drilledId !== null) {
        if (event.key === 'Escape') {
          event.preventDefault()
          handleBack()
        }
        return
      }
      if (navOrder.length === 0) return
      const current = effectiveSelected
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        const idx = current === null ? -1 : navOrder.indexOf(current)
        const next = navOrder[Math.min(idx + 1, navOrder.length - 1)]
        setSelectedId(next)
        focusRow(next)
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        const idx = current === null ? navOrder.length : navOrder.indexOf(current)
        const next = navOrder[Math.max(idx - 1, 0)]
        setSelectedId(next)
        focusRow(next)
      }
    },
    [drilledId, effectiveSelected, focusRow, handleBack, navOrder]
  )

  return {
    asideRef,
    selectedId,
    setSelectedId,
    drilledId,
    focusTimelineFor,
    effectiveSelected,
    drilledStory,
    renderedDrill,
    handleOpen,
    handleOpenTimeline,
    handleBack,
    handleDrillUnmountReady,
    handleKeyDown
  }
}
