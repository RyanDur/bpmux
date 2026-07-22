import { cleanup, render, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import Roadmap from './Roadmap'
import { roadmap } from './Roadmap.page'
import { card, seedRoadmap } from './Roadmap.fixtures'
import { resetStateForTest } from '../state/store'

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('#1704 Roadmap — outcomes by horizon, the overview', () => {
  it('groups outcomes under Now / Next / Later band headings', () => {
    seedRoadmap([
      card({ id: 1, name: 'Now outcome', horizon: 'now' }),
      card({ id: 2, name: 'Next outcome', horizon: 'next' }),
      card({ id: 3, name: 'Later outcome', horizon: 'later' })
    ])
    render(<Roadmap />)
    expect(roadmap.bandHeadings()).toEqual(['Now', 'Next', 'Later'])
    expect(within(roadmap.band('Now')).getByText('Now outcome')).toBeTruthy()
  })

  it('renders the plain outcome name, never substrate-jargon', () => {
    seedRoadmap([card({ id: 42, name: 'The team runs on its own infra' })])
    render(<Roadmap />)
    const el = roadmap.card('The team runs on its own infra')
    expect(within(el).getByText('The team runs on its own infra')).toBeTruthy()
    expect(el.textContent).not.toContain('#42')
    expect(el.textContent?.toLowerCase()).not.toContain('epic')
  })

  it('shows the benefit as the why-line, omitting it when absent', () => {
    seedRoadmap([
      card({ id: 1, name: 'Has benefit', why: { benefit: 'any agent plugs in' } }),
      card({ id: 2, name: 'No benefit', why: {} })
    ])
    render(<Roadmap />)
    expect(within(roadmap.card('Has benefit')).getByText('any agent plugs in')).toBeTruthy()
    expect(roadmap.hasBenefit('No benefit')).toBe(false)
  })

  it('renders only present meta fields and OMITS absent ones (no dash-for-absent)', () => {
    seedRoadmap([card({ id: 1, name: 'Meta outcome', why: { benefit: 'b', confidence: 'High' } })])
    render(<Roadmap />)
    const el = roadmap.card('Meta outcome')
    expect(within(el).getByText('Confidence')).toBeTruthy()
    expect(within(el).getByText('High')).toBeTruthy()
    expect(within(el).queryByText('Metric')).toBeNull()
    expect(within(el).queryByText('Value')).toBeNull()
    expect(el.textContent).not.toContain('—')
  })

  it('renders "—" only for a PRESENT-but-empty meta field (graceful no-parse-fail)', () => {
    seedRoadmap([card({ id: 1, name: 'Empty value', why: { value: '' } })])
    render(<Roadmap />)
    const el = roadmap.card('Empty value')
    expect(within(el).getByText('Value')).toBeTruthy()
    expect(within(el).getByText('—')).toBeTruthy()
  })

  it('renders the 3 convene-readiness chip states (ready, N-gap, and none = no chip)', () => {
    seedRoadmap([
      card({ id: 1, name: 'Ready one', state: 'ready' }),
      card({ id: 2, name: 'Gap two', state: 'needs-attention', gap_count: 1 }),
      card({ id: 3, name: 'None three', state: 'planned' })
    ])
    render(<Roadmap />)
    expect(roadmap.readiness('Ready one')?.textContent).toContain('ready')
    expect(roadmap.readiness('Gap two')?.textContent).toContain('1 gap')
    expect(roadmap.readiness('None three')).toBeNull()
  })

  it('shows the live indicator (readiness is live-derived, not a static dump)', () => {
    seedRoadmap([card({ id: 1 })])
    render(<Roadmap />)
    expect(roadmap.liveShown()).toBe(true)
  })

  it('renders an empty-state when there are no outcomes', () => {
    seedRoadmap([])
    render(<Roadmap />)
    expect(roadmap.emptyShown()).toBe(true)
  })
})
