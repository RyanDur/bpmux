import { describe, expect, test } from 'vitest'
import { deriveOutcomeDrillIn } from './deriveOutcomeDrillIn'
import type { BoardStory, IterationReadiness } from './types'

const readiness = (
  drafted: boolean,
  stories_ordered: boolean,
  review_closed: boolean,
  dev_ok: boolean
): IterationReadiness => ({ drafted, stories_ordered, review_closed, dev_ok })

const iteration = (
  number: number,
  title: string,
  position: number,
  checks: IterationReadiness,
  outcomeNumber = 100
): BoardStory => ({
  number,
  title,
  state: 'in-progress',
  labels: ['iteration'],
  assignees: [],
  created_at: '2026-01-01T00:00:00Z',
  body: 'goal',
  outcome_membership: { outcome_number: outcomeNumber, position },
  readiness: checks
})

describe('#1737 deriveOutcomeDrillIn — an outcome → the iterations building it, in order', () => {
  test('groups the building iterations under the outcome, in position order — not other outcomes, not epics/features', () => {
    const stories: BoardStory[] = [
      iteration(20, 'Lyra', 2, readiness(false, false, false, false)),
      iteration(10, 'Crux', 1, readiness(true, true, false, false)),
      iteration(99, 'Other', 1, readiness(true, true, true, true), 200),
      {
        number: 50,
        title: 'An epic',
        state: 'in-progress',
        labels: ['epic'],
        assignees: [],
        created_at: '2026-01-01T00:00:00Z',
        body: ''
      }
    ]

    const drill = deriveOutcomeDrillIn(100, stories)

    expect(drill.iterations.map((i) => i.name)).toEqual(['Crux', 'Lyra'])
    expect(drill.iterations.map((i) => i.order)).toEqual([1, 2])
  })

  test('a started-but-not-ready iteration carries tri-state checks: met ✓, the FIRST unmet is the active gap, later unmet are pending', () => {
    const stories = [iteration(10, 'Crux', 1, readiness(true, true, false, false))]

    const { iterations } = deriveOutcomeDrillIn(100, stories)
    const crux = iterations[0]

    expect(crux.state).toBe('next')
    expect(crux.gapCount).toBe(2)
    expect(crux.checks.map((c) => [c.label, c.state])).toEqual([
      ['drafted', 'met'],
      ['stories ordered', 'met'],
      ['designer-review', 'gap'],
      ['dev-OK', 'pending']
    ])
  })

  test('a met check AFTER an earlier gap still reads met — only the first unmet is the active gap', () => {
    const stories = [iteration(10, 'Crux', 1, readiness(false, true, true, false))]

    const checks = deriveOutcomeDrillIn(100, stories).iterations[0].checks
    expect(checks.map((c) => c.state)).toEqual(['gap', 'met', 'met', 'pending'])
  })

  test('an iteration with nothing done (all checks unmet) is not-started — no checks row', () => {
    const stories = [iteration(20, 'Lyra', 1, readiness(false, false, false, false))]

    const lyra = deriveOutcomeDrillIn(100, stories).iterations[0]
    expect(lyra.state).toBe('not-started')
    expect(lyra.checks).toEqual([])
  })

  test('an all-met iteration is ready, with every check met', () => {
    const stories = [iteration(10, 'Crux', 1, readiness(true, true, true, true))]

    const crux = deriveOutcomeDrillIn(100, stories).iterations[0]
    expect(crux.state).toBe('ready')
    expect(crux.gapCount).toBe(0)
    expect(crux.checks.every((c) => c.state === 'met')).toBe(true)
  })

  test('the CONVENE banner names the next started-not-ready iteration, its active gap, and the one after', () => {
    const stories = [
      iteration(10, 'Crux', 1, readiness(true, true, false, false)),
      iteration(20, 'Lyra', 2, readiness(false, false, false, false))
    ]

    const { convene } = deriveOutcomeDrillIn(100, stories)
    expect(convene).toEqual({
      nextIterationName: 'Crux',
      nextGapCount: 2,
      activeGapLabel: 'designer-review',
      thenIterationName: 'Lyra'
    })
  })

  test('when every building iteration is ready, there is nothing to convene', () => {
    const stories = [
      iteration(10, 'Crux', 1, readiness(true, true, true, true)),
      iteration(20, 'Lyra', 2, readiness(true, true, true, true))
    ]

    expect(deriveOutcomeDrillIn(100, stories).convene).toBeNull()
  })

  test('an outcome with no building iterations yields an empty drill', () => {
    const drill = deriveOutcomeDrillIn(100, [])
    expect(drill.iterations).toEqual([])
    expect(drill.convene).toBeNull()
  })
})
