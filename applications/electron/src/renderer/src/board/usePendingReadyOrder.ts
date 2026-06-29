import { useCallback, useMemo, useState } from 'react'
import type { Story } from './types'

export interface UsePendingReadyOrderResult {
  setPendingReadyOrder: (storyNumber: number, position: number) => void
  clearPendingReadyOrder: (storyNumbers: number[]) => void
  effectiveReadyPosition: (story: Story) => number | null
}

export const usePendingReadyOrder = (data: Story[] | null): UsePendingReadyOrderResult => {
  const stories = useMemo(() => data ?? [], [data])
  const [pending, setPending] = useState<Record<number, number>>({})

  const effective = useMemo<Record<number, number>>(() => {
    const out: Record<number, number> = {}
    for (const [numStr, position] of Object.entries(pending)) {
      const num = Number(numStr)
      const story = stories.find((s) => s.id === num)
      const actual = story?.iteration_membership?.position ?? null
      if (actual === position) continue
      out[num] = position
    }
    return out
  }, [stories, pending])

  const setPendingReadyOrder = useCallback((storyNumber: number, position: number): void => {
    setPending((prev) => ({ ...prev, [storyNumber]: position }))
  }, [])

  const clearPendingReadyOrder = useCallback((storyNumbers: number[]): void => {
    setPending((prev) => {
      let changed = false
      const next = { ...prev }
      for (const storyNumber of storyNumbers) {
        if (storyNumber in next) {
          delete next[storyNumber]
          changed = true
        }
      }
      return changed ? next : prev
    })
  }, [])

  const effectiveReadyPosition = useCallback(
    (story: Story): number | null => {
      const held = effective[story.id]
      if (held !== undefined) return held
      return story.iteration_membership?.position ?? null
    },
    [effective]
  )

  return { setPendingReadyOrder, clearPendingReadyOrder, effectiveReadyPosition }
}
