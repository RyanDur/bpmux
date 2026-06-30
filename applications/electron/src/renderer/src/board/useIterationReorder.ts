import { useDispatch } from '../transport/useDispatch'
import { iterationReorderRequested } from '../transport/session/actions'
import { eagerReorder, type IterationDragPayload } from './iterationMembers'
import type { BoardIterationData } from './selectIterations'

interface UseIterationReorderResult {
  reorderIterationByDrag: (payload: IterationDragPayload, targetKey: number) => void
  reorderIterationByKeyboard: (iteration: BoardIterationData, direction: 'up' | 'down') => void
}

export const useIterationReorder = (
  activePlannedIterations: BoardIterationData[],
  onReorderAnnounce?: (text: string) => void
): UseIterationReorderResult => {
  const dispatch = useDispatch()

  const placeIteration = (movedId: number, targetMemberIndex: number): number | null => {
    if (!activePlannedIterations.some((it) => it.id === movedId)) return null
    const targetId = activePlannedIterations[targetMemberIndex]?.id ?? null
    const newSequence = eagerReorder(activePlannedIterations, (it) => it.id, movedId, targetId)
    const finalIndex = newSequence.findIndex((it) => it.id === movedId)
    if (finalIndex === -1) return null
    const newPosition = finalIndex + 1
    dispatch(iterationReorderRequested(movedId, newPosition))
    return newPosition
  }

  const reorderIterationByDrag = (payload: IterationDragPayload, targetKey: number): void => {
    const targetIndex = activePlannedIterations.findIndex((it) => it.id === targetKey)
    if (targetIndex === -1) return
    placeIteration(payload.iterationId, targetIndex)
  }

  const reorderIterationByKeyboard = (
    iteration: BoardIterationData,
    direction: 'up' | 'down'
  ): void => {
    const iterationIdx = activePlannedIterations.findIndex((it) => it.id === iteration.id)
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
    const newPosition = placeIteration(iteration.id, targetMemberIndex)
    if (newPosition === null) return
    onReorderAnnounce?.(
      `Moved ${iteration.title} to position ${newPosition} of ${activePlannedIterations.length} among iterations`
    )
  }

  return { reorderIterationByDrag, reorderIterationByKeyboard }
}
