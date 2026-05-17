import { useCallback, useEffect, useMemo, useState } from 'react'
import type { BoardStory } from './types'
import {
  type PendingMemberPosition,
  isIterationStory,
  isLooseStory,
  positionInIteration,
  positionInIterationOrder,
  positionInUnscheduled
} from './iterationMembers'

export interface IterationMember {
  story: BoardStory
  position: number
}

export interface UseIterationMembersResult {
  sortedIterations: BoardStory[]
  sortedLoose: BoardStory[]
  getMembersFor: (themeName: string) => IterationMember[]
  setPending: (storyNumber: number, theme: string, position: number) => void
}

const compareIterations = (a: BoardStory, b: BoardStory): number => {
  const ao = positionInIterationOrder(a)
  const bo = positionInIterationOrder(b)
  if (ao !== null && bo !== null) return ao - bo
  if (ao !== null) return -1
  if (bo !== null) return 1
  return a.number - b.number
}

const compareLoose = (a: BoardStory, b: BoardStory): number => {
  const ap = positionInUnscheduled(a)
  const bp = positionInUnscheduled(b)
  if (ap !== null && bp !== null) return ap - bp
  if (ap !== null) return -1
  if (bp !== null) return 1
  return a.number - b.number
}

export const useIterationMembers = (
  stories: BoardStory[]
): UseIterationMembersResult => {
  const [pendingMemberPosition, setPendingMemberPosition] = useState<
    Record<number, PendingMemberPosition>
  >({})

  useEffect(() => {
    setPendingMemberPosition((prev) => {
      const next: Record<number, PendingMemberPosition> = {}
      let changed = false
      for (const [numStr, pending] of Object.entries(prev)) {
        const num = Number(numStr)
        const story = stories.find((s) => s.number === num)
        const actualPosition = story
          ? positionInIteration(story, pending.theme)
          : null
        if (actualPosition === pending.position) {
          changed = true
          continue
        }
        next[num] = pending
      }
      return changed ? next : prev
    })
  }, [stories])

  const sortedIterations = useMemo(
    () => [...stories.filter(isIterationStory)].sort(compareIterations),
    [stories]
  )

  const sortedLoose = useMemo(
    () => [...stories.filter(isLooseStory)].sort(compareLoose),
    [stories]
  )

  const getMembersFor = useCallback(
    (themeName: string): IterationMember[] =>
      stories
        .map((s) => {
          if (isIterationStory(s)) return null
          const pending = pendingMemberPosition[s.number]
          const labelPosition = positionInIteration(s, themeName)
          const pendingPosition =
            pending !== undefined && pending.theme === themeName
              ? pending.position
              : null
          if (pendingPosition !== null) {
            return { story: s, position: pendingPosition }
          }
          if (labelPosition === null) return null
          if (
            pending !== undefined &&
            pending.theme !== themeName &&
            pending.theme.toLowerCase() !== themeName.toLowerCase()
          ) {
            return null
          }
          return { story: s, position: labelPosition }
        })
        .filter((m): m is IterationMember => m !== null)
        .sort((a, b) => a.position - b.position),
    [stories, pendingMemberPosition]
  )

  const setPending = useCallback(
    (storyNumber: number, theme: string, position: number): void => {
      setPendingMemberPosition((prev) => ({
        ...prev,
        [storyNumber]: { theme, position }
      }))
    },
    []
  )

  return { sortedIterations, sortedLoose, getMembersFor, setPending }
}
