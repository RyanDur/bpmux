export const ITERATION_PREFIX = 'iteration-'
export const ITERATION_ORDER_PREFIX = 'iteration-order.'
export const PARENT_ITERATION_PREFIX = 'parent-iteration-#'
export const UNSCHEDULED_ITERATION_NAME = 'Unscheduled'
export const UNSCHEDULED_LABEL_PREFIX = `${ITERATION_PREFIX}${UNSCHEDULED_ITERATION_NAME}.`

const ITERATION_LABEL_RE = /^iteration-(.+)\.(\d+)$/
const PARENT_ITERATION_LABEL_RE = /^parent-iteration-#(\d+)\.(\d+)$/

export interface IterationLabelParts {
  name: string
  position: number
}

export interface ParentIterationLabelParts {
  iterationNumber: number
  position: number
}

export const iterationLabel = (name: string, position: number): string =>
  `${ITERATION_PREFIX}${name}.${position}`

export const parentIterationLabel = (iterationNumber: number, position: number): string =>
  `${PARENT_ITERATION_PREFIX}${iterationNumber}.${position}`

export const parseParentIterationLabel = (label: string): ParentIterationLabelParts | null => {
  const match = label.match(PARENT_ITERATION_LABEL_RE)
  if (match === null) return null
  return { iterationNumber: parseInt(match[1], 10), position: parseInt(match[2], 10) }
}

export const unscheduledLabel = (position: number): string =>
  `${UNSCHEDULED_LABEL_PREFIX}${position}`

export const parseIterationLabel = (label: string): IterationLabelParts | null => {
  const match = label.match(ITERATION_LABEL_RE)
  if (match === null) return null
  return { name: match[1], position: parseInt(match[2], 10) }
}

export const parseUnscheduledPosition = (label: string): number | null => {
  if (!label.startsWith(UNSCHEDULED_LABEL_PREFIX)) return null
  const n = parseInt(label.slice(UNSCHEDULED_LABEL_PREFIX.length), 10)
  return Number.isNaN(n) ? null : n
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
