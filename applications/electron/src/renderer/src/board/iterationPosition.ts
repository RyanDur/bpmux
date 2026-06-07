import { iterationOrderLabel, parentIterationLabel } from './labelSchema'

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

export interface OrderDropMutationInput {
  draggedOldPosition: number
  targetMemberPositions: number[]
  targetMemberIndex: number
  targetMembersIncludeDragged: boolean
}

export const computeIterationOrderMutation = (
  input: OrderDropMutationInput
): DropMutation | null => {
  const position = resolveReorderPosition(
    true,
    input.draggedOldPosition,
    input.targetMemberPositions,
    input.targetMemberIndex,
    input.targetMembersIncludeDragged
  )
  if (position === null) return null
  return {
    add: [iterationOrderLabel(position)],
    remove: [iterationOrderLabel(input.draggedOldPosition)]
  }
}

export interface MemberDropMutationInput {
  draggedIterationNumber: number
  draggedOldPosition: number
  targetIterationNumber: number
  targetMemberPositions: number[]
  targetMemberIndex: number
  targetMembersIncludeDragged: boolean
}

export interface IterationOrderSlot {
  number: number
  currentPosition: number | null
}

export interface IterationOrderAssignment extends DropMutation {
  iterationNumber: number
}

export const computeEstablishOrderMutations = (
  sequence: IterationOrderSlot[]
): IterationOrderAssignment[] => {
  const out: IterationOrderAssignment[] = []
  sequence.forEach((slot, index) => {
    const targetPosition = (index + 1) * DEFAULT_STEP
    if (slot.currentPosition === targetPosition) return
    out.push({
      iterationNumber: slot.number,
      add: [iterationOrderLabel(targetPosition)],
      remove: slot.currentPosition === null ? [] : [iterationOrderLabel(slot.currentPosition)]
    })
  })
  return out
}

export const computeMemberDropMutation = (input: MemberDropMutationInput): DropMutation | null => {
  const position = resolveReorderPosition(
    input.draggedIterationNumber === input.targetIterationNumber,
    input.draggedOldPosition,
    input.targetMemberPositions,
    input.targetMemberIndex,
    input.targetMembersIncludeDragged
  )
  if (position === null) return null
  return {
    add: [parentIterationLabel(input.targetIterationNumber, position)],
    remove: [parentIterationLabel(input.draggedIterationNumber, input.draggedOldPosition)]
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
  storyNumber: number
  currentPosition: number | null
}

export interface MemberOrderAssignment extends DropMutation {
  storyNumber: number
  position: number
}

export const computeMemberRespace = (
  iterationNumber: number,
  sequence: MemberSlot[]
): MemberOrderAssignment[] => {
  const out: MemberOrderAssignment[] = []
  sequence.forEach((slot, index) => {
    const targetPosition = (index + 1) * DEFAULT_STEP
    if (slot.currentPosition === targetPosition) return
    out.push({
      storyNumber: slot.storyNumber,
      position: targetPosition,
      add: [parentIterationLabel(iterationNumber, targetPosition)],
      remove:
        slot.currentPosition === null
          ? []
          : [parentIterationLabel(iterationNumber, slot.currentPosition)]
    })
  })
  return out
}
