import { describe, expect, it } from 'vitest'
import { selectIterations, type BoardIterationData } from './selectIterations'

const iteration = (over: Partial<BoardIterationData>): BoardIterationData => ({
  number: 1,
  title: 'Iteration X',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories: [],
  ...over
})

describe('selectIterations — the iteration domain collection, ordered', () => {
  it('orders by iteration_order, then number, sinking the unordered', () => {
    const ordered = selectIterations({
      iterations: [
        iteration({ number: 10, iteration_order: 2, title: 'second' }),
        iteration({ number: 20, iteration_order: 1, title: 'first' }),
        iteration({ number: 5, iteration_order: null, title: 'unordered' })
      ]
    })
    expect(ordered.map((i) => i.title)).toEqual(['first', 'second', 'unordered'])
  })
})
