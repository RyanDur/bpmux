import { parentIterationLabel } from './labelSchema'

export const DEFAULT_FIRST_POSITION = 100
export const DEFAULT_STEP = 100

export interface PositionFor {
  position: number
  cascadeNeeded: boolean
}

export const computeInsertPosition = (
  insertIndex: number,
  existingPositions: number[]
): PositionFor => {
  const positions = [...existingPositions].sort((a, b) => a - b)
  if (positions.length === 0) {
    return { position: DEFAULT_FIRST_POSITION, cascadeNeeded: false }
  }
  if (insertIndex <= 0) {
    const first = positions[0]
    if (first > 1) {
      const midpoint = Math.floor(first / 2)
      return { position: midpoint, cascadeNeeded: midpoint <= 0 }
    }
    return { position: 0, cascadeNeeded: true }
  }
  if (insertIndex >= positions.length) {
    const last = positions[positions.length - 1]
    return { position: last + DEFAULT_STEP, cascadeNeeded: false }
  }
  const left = positions[insertIndex - 1]
  const right = positions[insertIndex]
  const gap = right - left
  if (gap >= 2) {
    return { position: left + Math.floor(gap / 2), cascadeNeeded: false }
  }
  return { position: left, cascadeNeeded: true }
}

export interface DropMutation {
  add: string[]
  remove: string[]
}

const resolveReorderPosition = (
  sameContainer: boolean,
  draggedOldPosition: number,
  targetMemberPositions: number[],
  targetMemberIndex: number,
  targetMembersIncludeDragged: boolean
): number | null => {
  const withoutSource =
    sameContainer && targetMembersIncludeDragged
      ? targetMemberPositions.filter((p) => p !== draggedOldPosition)
      : targetMemberPositions
  const { position } = computeInsertPosition(targetMemberIndex, withoutSource)
  if (sameContainer && position === draggedOldPosition) return null
  return position
}

export interface MemberDropMutationInput {
  draggedIterationId: number
  draggedOldPosition: number
  targetIterationId: number
  targetMemberPositions: number[]
  targetMemberIndex: number
  targetMembersIncludeDragged: boolean
}

export const computeMemberDropMutation = (input: MemberDropMutationInput): DropMutation | null => {
  const position = resolveReorderPosition(
    input.draggedIterationId === input.targetIterationId,
    input.draggedOldPosition,
    input.targetMemberPositions,
    input.targetMemberIndex,
    input.targetMembersIncludeDragged
  )
  if (position === null) return null
  return {
    add: [parentIterationLabel(input.targetIterationId, position)],
    remove: [parentIterationLabel(input.draggedIterationId, input.draggedOldPosition)]
  }
}

export const insertPositionWouldCascade = (
  draggedOldPosition: number,
  targetMemberPositions: number[],
  targetMemberIndex: number
): boolean => {
  const withoutSource = targetMemberPositions.filter((p) => p !== draggedOldPosition)
  const { position, cascadeNeeded } = computeInsertPosition(targetMemberIndex, withoutSource)
  return cascadeNeeded || withoutSource.includes(position)
}

export interface MemberSlot {
  storyId: number
  currentPosition: number | null
}

export interface MemberOrderAssignment extends DropMutation {
  storyId: number
  position: number
}

export const computeMemberRespace = (
  iterationId: number,
  sequence: MemberSlot[]
): MemberOrderAssignment[] => {
  const out: MemberOrderAssignment[] = []
  sequence.forEach((slot, index) => {
    const targetPosition = (index + 1) * DEFAULT_STEP
    if (slot.currentPosition === targetPosition) return
    out.push({
      storyId: slot.storyId,
      position: targetPosition,
      add: [parentIterationLabel(iterationId, targetPosition)],
      remove:
        slot.currentPosition === null
          ? []
          : [parentIterationLabel(iterationId, slot.currentPosition)]
    })
  })
  return out
}
