export interface RowDragDropInput {
  fromIndex: number | null
  hoveredIndex: number
  clientY: number
  rectTop: number
  rectHeight: number
}

export const dropIndexForRowDrag = (input: RowDragDropInput): number => {
  const { fromIndex, hoveredIndex } = input
  if (fromIndex !== null && hoveredIndex === fromIndex + 1) {
    return hoveredIndex + 1
  }
  if (fromIndex !== null && hoveredIndex === fromIndex - 1) {
    return hoveredIndex
  }
  const mid = input.rectTop + input.rectHeight / 2
  return input.clientY < mid ? hoveredIndex : hoveredIndex + 1
}
