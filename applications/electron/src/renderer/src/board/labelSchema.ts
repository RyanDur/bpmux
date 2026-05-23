export const ITERATION_PREFIX = 'iteration-'
export const ITERATION_ORDER_PREFIX = 'iteration-order.'
export const READY_ORDER_PREFIX = 'ready-order.'
export const UNSCHEDULED_THEME_NAME = 'Unscheduled'
export const UNSCHEDULED_LABEL_PREFIX = `${ITERATION_PREFIX}${UNSCHEDULED_THEME_NAME}.`

const ITERATION_LABEL_RE = /^iteration-(.+)\.(\d+)$/

export interface IterationLabelParts {
  name: string
  position: number
}

export const iterationLabel = (themeName: string, position: number): string =>
  `${ITERATION_PREFIX}${themeName}.${position}`

export const readyOrderLabel = (position: number): string => `${READY_ORDER_PREFIX}${position}`

export const unscheduledLabel = (position: number): string =>
  `${UNSCHEDULED_LABEL_PREFIX}${position}`

export const parseIterationLabel = (label: string): IterationLabelParts | null => {
  const match = label.match(ITERATION_LABEL_RE)
  if (match === null) return null
  return { name: match[1], position: parseInt(match[2], 10) }
}

export const parseIterationOrderPosition = (label: string): number | null => {
  if (!label.startsWith(ITERATION_ORDER_PREFIX)) return null
  const n = parseInt(label.slice(ITERATION_ORDER_PREFIX.length), 10)
  return Number.isNaN(n) ? null : n
}

export const parseReadyOrderPosition = (label: string): number | null => {
  if (!label.startsWith(READY_ORDER_PREFIX)) return null
  const n = Number(label.slice(READY_ORDER_PREFIX.length))
  if (!Number.isFinite(n) || !Number.isInteger(n) || n < 0) return null
  return n
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

export const themeNameFromTitle = (title: string): string => {
  const match = title.match(/^Iteration (.+)$/)
  const raw = match !== null ? match[1].trim() : title.trim()
  return raw.replace(/\s+/g, '-')
}
