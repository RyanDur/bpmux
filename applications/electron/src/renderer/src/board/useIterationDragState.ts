import { useState } from 'react'

export interface UseIterationDragStateResult {
  draggedNumber: number | null
  setDraggedNumber: (n: number | null) => void
  hoveredNumber: number | null
  setHoveredNumber: (n: number | null) => void
  draggedIterationNumber: number | null
  setDraggedIterationNumber: (n: number | null) => void
  dragRejectedIterationNumber: number | null
  setDragRejectedIterationNumber: (n: number | null) => void
}

export const useIterationDragState = (): UseIterationDragStateResult => {
  const [draggedNumber, setDraggedNumber] = useState<number | null>(null)
  const [hoveredNumber, setHoveredNumber] = useState<number | null>(null)
  const [draggedIterationNumber, setDraggedIterationNumber] = useState<number | null>(
    null
  )
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<
    number | null
  >(null)
  return {
    draggedNumber,
    setDraggedNumber,
    hoveredNumber,
    setHoveredNumber,
    draggedIterationNumber,
    setDraggedIterationNumber,
    dragRejectedIterationNumber,
    setDragRejectedIterationNumber
  }
}
