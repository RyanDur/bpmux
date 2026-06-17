import { computeEstablishOrderMutations, computeIterationOrderMutation } from './iterationPosition'
import { parseIterationOrderPosition } from './labelSchema'
import {
  type IterationDragPayload,
  eagerReorder,
  iterationOrderLabelsOf,
  positionInIterationOrder,
  postLabelMutation
} from './iterationMembers'
import type { BoardStory } from './types'

interface UseIterationReorderResult {
  reorderIterationByDrag: (payload: IterationDragPayload, targetKey: number) => void
  reorderIterationByKeyboard: (iteration: BoardStory, direction: 'up' | 'down') => void
}

export const useIterationReorder = (
  activePlannedIterations: BoardStory[],
  setPendingIterationOrder: (iterationNumber: number, position: number) => void,
  onReorderAnnounce?: (text: string) => void
): UseIterationReorderResult => {
  const applyOrderMutation = (story: BoardStory, addLabel: string): void => {
    const remove = iterationOrderLabelsOf(story).filter((label) => label !== addLabel)
    postLabelMutation(story.number, [addLabel], remove)
    const newOrder = parseIterationOrderPosition(addLabel)
    if (newOrder !== null) {
      setPendingIterationOrder(story.number, newOrder)
    }
  }

  const placeIteration = (movedNumber: number, targetMemberIndex: number): number | null => {
    const moved = activePlannedIterations.find((it) => it.number === movedNumber)
    if (moved === undefined) return null
    const targetNumber = activePlannedIterations[targetMemberIndex]?.number ?? null
    const newSequence = eagerReorder(
      activePlannedIterations,
      (it) => it.number,
      movedNumber,
      targetNumber
    )
    const finalIndex = newSequence.findIndex((it) => it.number === movedNumber)
    if (finalIndex === -1) return null

    const draggedOldPosition = positionInIterationOrder(moved)
    const fullyOrdered = activePlannedIterations.every(
      (it) => positionInIterationOrder(it) !== null
    )

    if (fullyOrdered && draggedOldPosition !== null) {
      const activeOrders = activePlannedIterations
        .map((it) => positionInIterationOrder(it))
        .filter((p): p is number => p !== null)
      const mutation = computeIterationOrderMutation({
        draggedOldPosition,
        targetMemberPositions: activeOrders,
        targetMemberIndex,
        targetMembersIncludeDragged: true
      })
      if (mutation === null) return null
      applyOrderMutation(moved, mutation.add[0])
      return finalIndex + 1
    }

    const mutations = computeEstablishOrderMutations(
      newSequence.map((it) => ({
        number: it.number,
        currentPosition: positionInIterationOrder(it)
      }))
    )
    if (mutations.length === 0) return null
    for (const m of mutations) {
      const story = activePlannedIterations.find((it) => it.number === m.iterationNumber)
      if (story !== undefined) applyOrderMutation(story, m.add[0])
    }
    return finalIndex + 1
  }

  const reorderIterationByDrag = (payload: IterationDragPayload, targetKey: number): void => {
    const targetIndex = activePlannedIterations.findIndex((it) => it.number === targetKey)
    if (targetIndex === -1) return
    placeIteration(payload.iterationNumber, targetIndex)
  }

  const reorderIterationByKeyboard = (iteration: BoardStory, direction: 'up' | 'down'): void => {
    const iterationIdx = activePlannedIterations.findIndex((it) => it.number === iteration.number)
    if (iterationIdx === -1) return
    if (direction === 'up' && iterationIdx === 0) {
      onReorderAnnounce?.(`${iteration.title} is already at position 1 among iterations`)
      return
    }
    if (direction === 'down' && iterationIdx === activePlannedIterations.length - 1) {
      onReorderAnnounce?.(`${iteration.title} is already at last position among iterations`)
      return
    }
    const targetMemberIndex = direction === 'up' ? iterationIdx - 1 : iterationIdx + 1
    const newPosition = placeIteration(iteration.number, targetMemberIndex)
    if (newPosition === null) return
    onReorderAnnounce?.(
      `Moved ${iteration.title} to position ${newPosition} of ${activePlannedIterations.length} among iterations`
    )
  }

  return { reorderIterationByDrag, reorderIterationByKeyboard }
}
