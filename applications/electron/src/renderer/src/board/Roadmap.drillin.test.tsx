import { cleanup, render, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { buildingIteration, card, checks, outcome, seedRoadmap } from './Roadmap.fixtures'
import { resetStateForTest } from '../transport/state/store'

afterEach(() => {
  cleanup()
  resetStateForTest()
})

const OUTCOME = 'You see what is coming'

const seedSingle = (): void =>
  seedRoadmap(
    [card({ id: 1, name: OUTCOME, horizon: 'now' })],
    [
      outcome({
        id: 1,
        title: OUTCOME,
        horizon: 'now',
        iterations: [
          buildingIteration(10, 'Crux', 1, checks(true, true, false, false)),
          buildingIteration(20, 'Lyra', 2, checks(false, false, false, false))
        ]
      })
    ]
  )

describe('#1737 Roadmap drill-in — focus an outcome → its iterations building it, in order', () => {
  it('is collapsed by default — the drillbody is NOT in the DOM (render-on-demand)', () => {
    seedSingle()
    render(<Roadmap />)
    expect(roadmap.drillShown(OUTCOME)).toBe(false)
    expect(roadmap.iterationShown('Crux')).toBe(false)
  })

  it('focusing an outcome reveals the SEQUENCE OF ITERATIONS building it, in order, with the CONVENE? banner', async () => {
    seedSingle()
    render(<Roadmap />)
    await roadmap.focus(OUTCOME)
    expect(roadmap.drillIterationNames(OUTCOME)).toEqual(['Crux', 'Lyra'])
    const convene = roadmap.convene(OUTCOME)
    expect(convene).toContain('CONVENE?')
    expect(convene).toContain('Crux')
    expect(convene).toContain('designer-review')
  })

  it('the focused iteration shows its tri-state convene-checks; a not-started iteration shows none', async () => {
    seedSingle()
    render(<Roadmap />)
    await roadmap.focus(OUTCOME)
    const crux = roadmap.drillIteration('Crux')
    expect(within(crux).getByText('designer-review')).toBeTruthy()
    expect(within(crux).getByText('drafted')).toBeTruthy()
    const lyra = roadmap.drillIteration('Lyra')
    expect(lyra.getAttribute('data-state')).toBe('not-started')
    expect(roadmap.checksShown('Lyra')).toBe(false)
  })

  it('toggling the focused outcome again collapses it — back to the overview', async () => {
    seedSingle()
    render(<Roadmap />)
    await roadmap.focus(OUTCOME)
    expect(roadmap.drill(OUTCOME)).toBeTruthy()
    await roadmap.focus(OUTCOME)
    expect(roadmap.drillShown(OUTCOME)).toBe(false)
  })

  it('focuses one outcome at a time — focusing another collapses the first', async () => {
    seedRoadmap(
      [
        card({ id: 1, name: 'first', horizon: 'now' }),
        card({ id: 2, name: 'second', horizon: 'now' })
      ],
      [
        outcome({
          id: 1,
          title: 'first',
          horizon: 'now',
          iterations: [buildingIteration(10, 'Crux', 1, checks(true, false, false, false))]
        }),
        outcome({
          id: 2,
          title: 'second',
          horizon: 'now',
          iterations: [buildingIteration(30, 'Vega', 1, checks(true, false, false, false))]
        })
      ]
    )
    render(<Roadmap />)
    await roadmap.focus('first')
    expect(roadmap.drill('first')).toBeTruthy()
    await roadmap.focus('second')
    expect(roadmap.drillShown('first')).toBe(false)
    expect(roadmap.drill('second')).toBeTruthy()
  })
})
