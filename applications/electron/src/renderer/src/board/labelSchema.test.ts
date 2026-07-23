import { describe, expect, it } from 'vitest'

import {
  ITERATION_ORDER_PREFIX,
  ITERATION_PREFIX,
  iterationLabel,
  iterationNameFromTitle,
  isIterationPositionLabel
} from './labelSchema'

describe('labelSchema — single-source-of-truth constants + parsers', () => {
  it('iteration label constructor mirrors prefix + position', () => {
    expect(iterationLabel('Andromeda', 200)).toBe('iteration-Andromeda.200')
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
  })
})
