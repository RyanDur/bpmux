import { cleanup, render, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { buildingIteration, checks, outcome } from './Roadmap.fixtures'
import type { BoardStory } from './types'

afterEach(() => cleanup())

const stories = (): BoardStory[] => [
  outcome({ number: 1, title: 'You see what is coming', horizon: 'now' }),
  buildingIteration(10, 'Crux', 1, checks(true, true, false, false)),
  buildingIteration(20, 'Lyra', 2, checks(false, false, false, false)),
  { ...outcome({ number: 2 }), labels: ['epic'], title: 'a board epic' }
]

describe('#1737 Roadmap drill-in — focus an outcome → its iterations building it, in order', () => {
  it('is collapsed by default — the drillbody is NOT in the DOM (render-on-demand)', () => {
    render(<Roadmap stories={stories()} />)
    expect(roadmap.isShown('roadmap-drill-1')).toBe(false)
    expect(roadmap.isShown('roadmap-drill-iteration-10')).toBe(false)
  })

  it('focusing an outcome reveals the SEQUENCE OF ITERATIONS building it, in order, with the CONVENE? banner', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(1)
    expect(roadmap.drillIterationNames(1)).toEqual(['Crux', 'Lyra'])
    const convene = roadmap.convene(1)
    expect(convene).toContain('CONVENE?')
    expect(convene).toContain('Crux')
    expect(convene).toContain('designer-review')
  })

  it('the focused iteration shows its tri-state convene-checks; a not-started iteration shows none', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(1)
    const crux = roadmap.drillIteration(10)
    expect(within(crux).getByText('designer-review')).toBeTruthy()
    expect(within(crux).getByText('drafted')).toBeTruthy()
    const lyra = roadmap.drillIteration(20)
    expect(lyra.getAttribute('data-state')).toBe('not-started')
    expect(roadmap.isShown('roadmap-drill-checks-20')).toBe(false)
  })

  it('surfaces ONLY the iterations building the outcome — never epics (a board grouping)', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(1)
    expect(roadmap.drill(1).textContent).not.toContain('a board epic')
  })

  it('toggling the focused outcome again collapses it — back to the overview', async () => {
    render(<Roadmap stories={stories()} />)
    await roadmap.focus(1)
    expect(roadmap.drill(1)).toBeTruthy()
    await roadmap.focus(1)
    expect(roadmap.isShown('roadmap-drill-1')).toBe(false)
  })

  it('focuses one outcome at a time — focusing another collapses the first', async () => {
    const multi: BoardStory[] = [
      outcome({ number: 1, title: 'first', horizon: 'now' }),
      buildingIteration(10, 'Crux', 1, checks(true, false, false, false), 1),
      outcome({ number: 2, title: 'second', horizon: 'now' }),
      buildingIteration(30, 'Vega', 1, checks(true, false, false, false), 2)
    ]
    render(<Roadmap stories={multi} />)
    await roadmap.focus(1)
    expect(roadmap.drill(1)).toBeTruthy()
    await roadmap.focus(2)
    expect(roadmap.isShown('roadmap-drill-1')).toBe(false)
    expect(roadmap.drill(2)).toBeTruthy()
  })
})
