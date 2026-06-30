import { useCallback, useMemo, useState } from 'react'
import { type Story } from './types'
import type { BoardIterationData } from './selectIterations'
import {
  type PendingMemberPosition,
  buildMembershipIndex,
  isIterationStory,
  isLooseStory,
  positionInParentIteration,
  positionInUnscheduled
} from './iterationMembers'

export interface IterationMember {
  story: Story
  position: number
}

export interface UseIterationMembersResult {
  sortedLoose: Story[]
  getMembersFor: (iterationId: number) => IterationMember[]
  setPending: (storyId: number, iterationId: number, position: number) => void
}

const compareLoose = (a: Story, b: Story): number => {
  const ap = positionInUnscheduled(a)
  const bp = positionInUnscheduled(b)
  if (ap !== null && bp !== null) return ap - bp
  if (ap !== null) return -1
  if (bp !== null) return 1
  return a.id - b.id
}

export const useIterationMembers = (
  stories: Story[],
  iterations: BoardIterationData[]
): UseIterationMembersResult => {
  const [pendingMemberPosition, setPendingMemberPosition] = useState<
    Record<number, PendingMemberPosition>
  >({})

  const index = useMemo(() => buildMembershipIndex(iterations), [iterations])
  const storiesByNumber = useMemo(() => {
    const map = new Map<number, Story>()
    for (const s of stories) map.set(s.id, s)
    return map
  }, [stories])
  const iterationsByNumber = useMemo(() => {
    const map = new Map<number, BoardIterationData>()
    for (const it of iterations) map.set(it.id, it)
    return map
  }, [iterations])

  const effectivePending = useMemo<Record<number, PendingMemberPosition>>(() => {
    const out: Record<number, PendingMemberPosition> = {}
    for (const [numStr, pending] of Object.entries(pendingMemberPosition)) {
      const num = Number(numStr)
      const actualPosition = positionInParentIteration(index, num, pending.iterationId)
      if (actualPosition === pending.position) continue
      out[num] = pending
    }
    return out
  }, [index, pendingMemberPosition])

  const sortedLoose = useMemo(
    () => [...stories.filter((s) => isLooseStory(s, index))].sort(compareLoose),
    [stories, index]
  )

  const getMembersFor = useCallback(
    (iterationId: number): IterationMember[] => {
      const iteration = iterationsByNumber.get(iterationId)
      if (!iteration) return []
      let order = iteration.stories.filter((n) => {
        const s = storiesByNumber.get(n)
        if (!s || isIterationStory(s)) return false
        const pending = effectivePending[n]
        return !(pending && pending.iterationId !== iterationId)
      })
      for (const [numStr, pending] of Object.entries(effectivePending)) {
        if (pending.iterationId !== iterationId) continue
        const num = Number(numStr)
        const s = storiesByNumber.get(num)
        if (!s || isIterationStory(s)) continue
        order = order.filter((x) => x !== num)
        const idx = Math.min(Math.max(pending.position - 1, 0), order.length)
        order.splice(idx, 0, num)
      }
      return order.map((n, i) => ({ story: storiesByNumber.get(n) as Story, position: i + 1 }))
    },
    [iterationsByNumber, storiesByNumber, effectivePending]
  )

  const setPending = useCallback((storyId: number, iterationId: number, position: number): void => {
    setPendingMemberPosition((prev) => ({
      ...prev,
      [storyId]: { iterationId, position }
    }))
  }, [])

  return { sortedLoose, getMembersFor, setPending }
}
