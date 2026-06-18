import { cleanup, render, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { buildingIteration, checks, outcome } from './Roadmap.fixtures'
import type { BoardStory } from './types'

afterEach(() => cleanup())

const OUTCOME = 'You see what is coming'

const stories = (): BoardStory[] => [
  outcome({ number: 1, title: OUTCOME, horizon: 'now' }),
  buildingIteration(10, 'Crux', 1, checks(true, true, false, false)),
  buildingIteration(20, 'Lyra', 2, checks(false, false, false, false)),
  { ...outcome({ number: 2 }), labels: ['epic'], title: 'a board epic' }
]

describe('#1737 Roadmap drill-in — focus an outcome → its iterations building it, in order', () => {
  it('is collapsed by default — the drillbody is NOT in the DOM (render-on-demand)', () => {
    render(<Roadmap stories={stories()} />)
    expect(roadmap.drillShown(OUTCOME)).toBe(false)
    expect(roadmap.iterationShown('Crux')).toBe(false)
  })

  it('focusing an outcome reveals the SEQUENCE OF ITERATIONS building it, in order, with the CONVENE? banner', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(OUTCOME)
    expect(roadmap.drillIterationNames(OUTCOME)).toEqual(['Crux', 'Lyra'])
    const convene = roadmap.convene(OUTCOME)
    expect(convene).toContain('CONVENE?')
    expect(convene).toContain('Crux')
    expect(convene).toContain('designer-review')
  })

  it('the focused iteration shows its tri-state convene-checks; a not-started iteration shows none', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(OUTCOME)
    const crux = roadmap.drillIteration('Crux')
    expect(within(crux).getByText('designer-review')).toBeTruthy()
    expect(within(crux).getByText('drafted')).toBeTruthy()
    const lyra = roadmap.drillIteration('Lyra')
    expect(lyra.getAttribute('data-state')).toBe('not-started')
    expect(roadmap.checksShown('Lyra')).toBe(false)
  })

  it('surfaces ONLY the iterations building the outcome — never epics (a board grouping)', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(OUTCOME)
    expect(roadmap.drill(OUTCOME).textContent).not.toContain('a board epic')
  })

  it('toggling the focused outcome again collapses it — back to the overview', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(OUTCOME)
    expect(roadmap.drill(OUTCOME)).toBeTruthy()
    await roadmap.focus(OUTCOME)
    expect(roadmap.drillShown(OUTCOME)).toBe(false)
  })

  it('focuses one outcome at a time — focusing another collapses the first', async () => {
    const multi: BoardStory[] = [
      outcome({ number: 1, title: 'first', horizon: 'now' }),
      buildingIteration(10, 'Crux', 1, checks(true, false, false, false), 1),
      outcome({ number: 2, title: 'second', horizon: 'now' }),
      buildingIteration(30, 'Vega', 1, checks(true, false, false, false), 2)
    ]
    render(<Roadmap stories={multi} />)
    await roadmap.focus('first')
    expect(roadmap.drill('first')).toBeTruthy()
    await roadmap.focus('second')
    expect(roadmap.drillShown('first')).toBe(false)
    expect(roadmap.drill('second')).toBeTruthy()
  })
})
