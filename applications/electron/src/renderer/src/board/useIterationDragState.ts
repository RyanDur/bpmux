import { useState } from 'react'

export interface UseIterationDragStateResult {
  draggedIterationNumber: number | null
  setDraggedIterationNumber: (n: number | null) => void
  dragRejectedIterationNumber: number | null
  setDragRejectedIterationNumber: (n: number | null) => void
}

export const useIterationDragState = (): UseIterationDragStateResult => {
  const [draggedIterationNumber, setDraggedIterationNumber] = useState<number | null>(null)
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<number | null>(
    null
  )
  return {
    draggedIterationNumber,
    setDraggedIterationNumber,
    dragRejectedIterationNumber,
    setDragRejectedIterationNumber
  }
}
