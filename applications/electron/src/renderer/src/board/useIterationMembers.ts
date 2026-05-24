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

export const useIterationMembers = (stories: BoardStory[]): UseIterationMembersResult => {
  const [pendingMemberPosition, setPendingMemberPosition] = useState<
    Record<number, PendingMemberPosition>
  >({})

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

  const sortedIterations = useMemo(
    () => [...stories.filter(isIterationStory)].sort(compareIterations),
    [stories]
  )

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

  return { sortedIterations, sortedLoose, getMembersFor, setPending }
}
