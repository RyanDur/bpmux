export const ITERATION_PREFIX = 'iteration-'
export const ITERATION_ORDER_PREFIX = 'iteration-order.'
export const PARENT_ITERATION_PREFIX = 'parent-iteration-#'

const ITERATION_LABEL_RE = /^iteration-(.+)\.(\d+)$/
const PARENT_ITERATION_LABEL_RE = /^parent-iteration-#(\d+)\.(\d+)$/

export interface ParentIterationLabelParts {
  iterationId: number
  position: number
}

export const iterationLabel = (name: string, position: number): string =>
  `${ITERATION_PREFIX}${name}.${position}`

export const parentIterationLabel = (iterationId: number, position: number): string =>
  `${PARENT_ITERATION_PREFIX}${iterationId}.${position}`

export const parseParentIterationLabel = (label: string): ParentIterationLabelParts | null => {
  const match = label.match(PARENT_ITERATION_LABEL_RE)
  if (match === null) return null
  return { iterationId: parseInt(match[1], 10), position: parseInt(match[2], 10) }
}

export const isIterationPositionLabel = (label: string): boolean => {
  if (!label.startsWith(ITERATION_PREFIX)) return false
  if (label.startsWith(ITERATION_ORDER_PREFIX)) return false
  return ITERATION_LABEL_RE.test(label)
}

export const iterationNameFromTitle = (title: string): string => {
  const match = title.match(/^Iteration (.+)$/)
  const raw = match !== null ? match[1].trim() : title.trim()
  return raw.replace(/\s+/g, '-')
}
