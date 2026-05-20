import { useCallback, useEffect, useRef, useState } from 'react'
import { isNearTopUnderColumnReverse } from '../chat/scrollMath'

const BOTTOM_THRESHOLD_PX = 8

interface ScrollGutterArgs {
  visibleCount: number
  loadOlder?: () => Promise<void>
  isLoadingOlder: boolean
  hasMoreOlder: boolean
  onAtBottomChange?: (atBottom: boolean) => void
}

interface ScrollGutterApi {
  listRef: React.RefObject<HTMLUListElement | null>
  handleScroll: () => void
  newCount: number
  scrollToBottom: () => void
}

export const useScrollGutter = ({
  visibleCount,
  loadOlder,
  isLoadingOlder,
  hasMoreOlder,
  onAtBottomChange
}: ScrollGutterArgs): ScrollGutterApi => {
  const listRef = useRef<HTMLUListElement | null>(null)
  const atBottomRef = useRef<boolean>(true)
  const lastLengthRef = useRef<number>(visibleCount)
  const [newCount, setNewCount] = useState<number>(0)

  const triggerableLoadOlder =
    loadOlder && hasMoreOlder && !isLoadingOlder ? loadOlder : null

  const handleScroll = useCallback((): void => {
    const ul = listRef.current
    if (!ul) return
    const atBottom = Math.abs(ul.scrollTop) <= BOTTOM_THRESHOLD_PX
    if (atBottom !== atBottomRef.current) {
      atBottomRef.current = atBottom
      onAtBottomChange?.(atBottom)
    }
    if (atBottom && newCount !== 0) setNewCount(0)
    if (
      triggerableLoadOlder &&
      isNearTopUnderColumnReverse(ul.scrollTop, ul.scrollHeight, ul.clientHeight)
    ) {
      void triggerableLoadOlder()
    }
  }, [newCount, triggerableLoadOlder, onAtBottomChange])

  useEffect(() => {
    const prev = lastLengthRef.current
    const curr = visibleCount
    lastLengthRef.current = curr
    if (curr <= prev) return
    if (atBottomRef.current) return
    setNewCount((n) => n + (curr - prev))
  }, [visibleCount])

  const scrollToBottom = useCallback((): void => {
    const ul = listRef.current
    if (!ul) return
    ul.scrollTo({ top: 0, behavior: 'smooth' })
    setNewCount(0)
  }, [])

  return { listRef, handleScroll, newCount, scrollToBottom }
}
