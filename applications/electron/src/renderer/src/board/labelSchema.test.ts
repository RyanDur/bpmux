import { describe, expect, it } from 'vitest'

import {
  ITERATION_ORDER_PREFIX,
  ITERATION_PREFIX,
  UNSCHEDULED_ITERATION_NAME,
  UNSCHEDULED_LABEL_PREFIX,
  iterationLabel,
  iterationNameFromTitle,
  isIterationPositionLabel,
  parseIterationLabel,
  parseUnscheduledPosition,
  unscheduledLabel
} from './labelSchema'

describe('#1365 labelSchema — single-source-of-truth constants + parsers', () => {
  it('iteration label constructor mirrors prefix + position', () => {
    expect(iterationLabel('Andromeda', 200)).toBe('iteration-Andromeda.200')
  })

  it('parses an iteration label into name + position', () => {
    expect(parseIterationLabel('iteration-Andromeda.500')).toEqual({
      name: 'Andromeda',
      position: 500
    })
  })

  it('rejects malformed iteration labels', () => {
    expect(parseIterationLabel('iteration-NoPosition')).toBeNull()
    expect(parseIterationLabel('not-a-label')).toBeNull()
  })

  it('parses unscheduled position from iteration-Unscheduled prefix', () => {
    expect(parseUnscheduledPosition('iteration-Unscheduled.42')).toBe(42)
    expect(parseUnscheduledPosition('iteration-Cassiopeia.42')).toBeNull()
  })

  it('unscheduledLabel constructor mirrors prefix + position', () => {
    expect(unscheduledLabel(100)).toBe('iteration-Unscheduled.100')
  })

  it('isIterationPositionLabel matches Name.N pattern', () => {
    expect(isIterationPositionLabel('iteration-Andromeda.100')).toBe(true)
    expect(isIterationPositionLabel('iteration-Big-Dipper.50')).toBe(true)
  })

  it('isIterationPositionLabel rejects iteration-order labels', () => {
    expect(isIterationPositionLabel('iteration-order.100')).toBe(false)
  })

  it('isIterationPositionLabel rejects non-iteration', () => {
    expect(isIterationPositionLabel('blocked')).toBe(false)
    expect(isIterationPositionLabel('ready')).toBe(false)
    expect(isIterationPositionLabel('iteration')).toBe(false)
  })

  it('iterationNameFromTitle strips iteration prefix + hyphenates whitespace', () => {
    expect(iterationNameFromTitle('Iteration Andromeda')).toBe('Andromeda')
    expect(iterationNameFromTitle('Iteration Big Dipper')).toBe('Big-Dipper')
    expect(iterationNameFromTitle('  raw  ')).toBe('raw')
  })

  it('exports the canonical prefix constants', () => {
    expect(ITERATION_PREFIX).toBe('iteration-')
    expect(ITERATION_ORDER_PREFIX).toBe('iteration-order.')
    expect(UNSCHEDULED_ITERATION_NAME).toBe('Unscheduled')
    expect(UNSCHEDULED_LABEL_PREFIX).toBe('iteration-Unscheduled.')
  })
})
