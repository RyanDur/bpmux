import { useCallback, useMemo, useRef, useState } from 'react'
import type { Story } from './types'

interface UseBoardNavigationResult {
  asideRef: React.RefObject<HTMLElement | null>
  selectedNumber: number | null
  setSelectedNumber: React.Dispatch<React.SetStateAction<number | null>>
  drilledNumber: number | null
  focusTimelineFor: number | null
  effectiveSelected: number | null
  drilledStory: Story | null
  renderedDrill: Story | null
  handleOpen: (number: number) => void
  handleOpenTimeline: (number: number) => void
  handleBack: () => void
  handleDrillUnmountReady: () => void
  handleKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void
}

export const useBoardNavigation = (
  stories: Story[],
  navOrder: number[]
): UseBoardNavigationResult => {
  const [selectedNumber, setSelectedNumber] = useState<number | null>(null)
  const [drilledNumber, setDrilledNumber] = useState<number | null>(null)
  const [focusTimelineFor, setFocusTimelineFor] = useState<number | null>(null)
  const asideRef = useRef<HTMLElement | null>(null)

  const effectiveSelected =
    selectedNumber !== null && navOrder.includes(selectedNumber)
      ? selectedNumber
      : (navOrder[0] ?? null)

  const focusRow = useCallback((number: number | null) => {
    if (number === null) return
    requestAnimationFrame(() => {
      asideRef.current
        ?.querySelector<HTMLElement>(`.board-story-navigate[aria-label^="Open story #${number} "]`)
        ?.focus()
    })
  }, [])

  const drilledStory = useMemo(
    () => (drilledNumber === null ? null : (stories.find((s) => s.id === drilledNumber) ?? null)),
    [drilledNumber, stories]
  )

  const [lastDrilledStory, setLastDrilledStory] = useState<typeof drilledStory>(drilledStory)
  if (drilledStory !== null && drilledStory !== lastDrilledStory) {
    setLastDrilledStory(drilledStory)
  }
  const renderedDrill = drilledStory ?? lastDrilledStory
  const handleDrillUnmountReady = useCallback(() => {
    if (drilledStory === null) setLastDrilledStory(null)
  }, [drilledStory])

  const handleOpen = useCallback((number: number) => {
    setSelectedNumber(number)
    setDrilledNumber(number)
    setFocusTimelineFor(null)
  }, [])

  const handleOpenTimeline = useCallback((number: number) => {
    setSelectedNumber(number)
    setDrilledNumber(number)
    setFocusTimelineFor(number)
  }, [])

  const handleBack = useCallback(() => {
    setDrilledNumber(null)
    setFocusTimelineFor(null)
    focusRow(effectiveSelected)
  }, [effectiveSelected, focusRow])

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      if (drilledNumber !== null) {
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
        setSelectedNumber(next)
        focusRow(next)
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        const idx = current === null ? navOrder.length : navOrder.indexOf(current)
        const next = navOrder[Math.max(idx - 1, 0)]
        setSelectedNumber(next)
        focusRow(next)
      }
    },
    [drilledNumber, effectiveSelected, focusRow, handleBack, navOrder]
  )

  return {
    asideRef,
    selectedNumber,
    setSelectedNumber,
    drilledNumber,
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
