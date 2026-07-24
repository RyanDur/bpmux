export const ITERATION_PREFIX = 'iteration-'
export const ITERATION_ORDER_PREFIX = 'iteration-order.'

const ITERATION_LABEL_RE = /^iteration-(.+)\.(\d+)$/

export const iterationLabel = (name: string, position: number): string =>
  `${ITERATION_PREFIX}${name}.${position}`

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
