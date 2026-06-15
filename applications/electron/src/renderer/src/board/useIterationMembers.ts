import { useCallback, useMemo, useState } from 'react'
import type { BoardStory } from './types'
import {
  type PendingMemberPosition,
  isIterationStory,
  isLooseStory,
  positionInIterationOrder,
  positionInParentIteration,
  positionInUnscheduled
} from './iterationMembers'

export interface IterationMember {
  story: BoardStory
  position: number
}

export interface UseIterationMembersResult {
  sortedIterations: BoardStory[]
  sortedLoose: BoardStory[]
  getMembersFor: (iterationNumber: number) => IterationMember[]
  setPending: (storyNumber: number, iterationNumber: number, position: number) => void
  setPendingIterationOrder: (iterationNumber: number, position: number) => void
}

const compareLoose = (a: BoardStory, b: BoardStory): number => {
  const ap = positionInUnscheduled(a)
  const bp = positionInUnscheduled(b)
  if (ap !== null && bp !== null) return ap - bp
  if (ap !== null) return -1
  if (bp !== null) return 1
  return a.number - b.number
}

export const useIterationMembers = (stories: BoardStory[]): UseIterationMembersResult => {
  const [pendingMemberPosition, setPendingMemberPosition] = useState<
    Record<number, PendingMemberPosition>
  >({})
  const [pendingIterationOrder, setPendingIterationOrderState] = useState<Record<number, number>>(
    {}
  )

  const effectivePending = useMemo<Record<number, PendingMemberPosition>>(() => {
    const out: Record<number, PendingMemberPosition> = {}
    for (const [numStr, pending] of Object.entries(pendingMemberPosition)) {
      const num = Number(numStr)
      const story = stories.find((s) => s.number === num)
      const actualPosition = story
        ? positionInParentIteration(story, pending.iterationNumber)
        : null
      if (actualPosition === pending.position) continue
      out[num] = pending
    }
    return out
  }, [stories, pendingMemberPosition])

  const effectivePendingIterationOrder = useMemo<Record<number, number>>(() => {
    const out: Record<number, number> = {}
    for (const [numStr, position] of Object.entries(pendingIterationOrder)) {
      const num = Number(numStr)
      const story = stories.find((s) => s.number === num)
      const actualPosition = story ? positionInIterationOrder(story) : null
      if (actualPosition === position) continue
      out[num] = position
    }
    return out
  }, [stories, pendingIterationOrder])

  const sortedIterations = useMemo(() => {
    const orderOf = (story: BoardStory): number | null =>
      effectivePendingIterationOrder[story.number] ?? positionInIterationOrder(story)
    const compare = (a: BoardStory, b: BoardStory): number => {
      const ao = orderOf(a)
      const bo = orderOf(b)
      if (ao !== null && bo !== null) return ao - bo
      if (ao !== null) return -1
      if (bo !== null) return 1
      return 0
    }
    return [...stories.filter(isIterationStory)].sort(compare)
  }, [stories, effectivePendingIterationOrder])

  const sortedLoose = useMemo(() => [...stories.filter(isLooseStory)].sort(compareLoose), [stories])

  const getMembersFor = useCallback(
    (iterationNumber: number): IterationMember[] =>
      stories
        .map((s) => {
          if (isIterationStory(s)) return null
          const pending = effectivePending[s.number]
          const labelPosition = positionInParentIteration(s, iterationNumber)
          const pendingPosition =
            pending !== undefined && pending.iterationNumber === iterationNumber
              ? pending.position
              : null
          if (pendingPosition !== null) {
            return { story: s, position: pendingPosition }
          }
          if (labelPosition === null) return null
          if (pending !== undefined && pending.iterationNumber !== iterationNumber) {
            return null
          }
          return { story: s, position: labelPosition }
        })
        .filter((m): m is IterationMember => m !== null)
        .sort((a, b) => a.position - b.position),
    [stories, effectivePending]
  )

  const setPending = useCallback(
    (storyNumber: number, iterationNumber: number, position: number): void => {
      setPendingMemberPosition((prev) => ({
        ...prev,
        [storyNumber]: { iterationNumber, position }
      }))
    },
    []
  )

  const setPendingIterationOrder = useCallback(
    (iterationNumber: number, position: number): void => {
      setPendingIterationOrderState((prev) => ({ ...prev, [iterationNumber]: position }))
    },
    []
  )

  return { sortedIterations, sortedLoose, getMembersFor, setPending, setPendingIterationOrder }
}
