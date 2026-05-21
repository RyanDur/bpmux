import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Iteration from './Iteration'
import { iterationLabel } from './labelSchema'
import type { BoardStory } from './types'

const iterationStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9000,
  title: 'Iteration Lyra',
  state: 'icebox',
  gh_state: 'open',
  labels: ['Iteration'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const memberStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9100,
  title: 'member',
  state: 'icebox',
  gh_state: 'open',
  labels: ['Feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1413 Iteration — renders regardless of member-count or open/closed state', () => {
  test('Scenario 1 (fix-anchor): open + 0 members surfaces in icebox-iterations-list', () => {
    const lyra = iterationStory({ number: 9000, title: 'Iteration Lyra', gh_state: 'open' })
    const html = renderToStaticMarkup(<Iteration stories={[lyra]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9000"')
    expect(html).toContain('Iteration Lyra')
    expect(html).toContain('(0)')
  })

  test('Scenario 2 (regression-anchor): closed + 0 members surfaces with closed badge', () => {
    const walkVerify = iterationStory({
      number: 9001,
      title: 'Iteration WalkVerify',
      gh_state: 'closed'
    })
    const html = renderToStaticMarkup(<Iteration stories={[walkVerify]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9001"')
    expect(html).toContain('Iteration WalkVerify')
    expect(html).toContain('(closed)')
    expect(html).toContain('(0)')
  })

  test('Scenario 3 (regression-anchor): open + N members surfaces with member-count', () => {
    const carina = iterationStory({ number: 9002, title: 'Iteration Carina', gh_state: 'open' })
    const m1 = memberStory({ number: 9201, labels: ['Feature', iterationLabel('Carina', 100)] })
    const m2 = memberStory({ number: 9202, labels: ['Feature', iterationLabel('Carina', 200)] })
    const html = renderToStaticMarkup(<Iteration stories={[carina, m1, m2]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9002"')
    expect(html).toContain('Iteration Carina')
    expect(html).toContain('(2)')
    expect(html).not.toContain('(closed)')
  })

  test('Scenario 4 (regression-anchor): closed + N members surfaces with closed badge + count', () => {
    const andromeda = iterationStory({
      number: 9003,
      title: 'Iteration Andromeda',
      gh_state: 'closed'
    })
    const m1 = memberStory({ number: 9301, labels: ['Feature', iterationLabel('Andromeda', 100)] })
    const html = renderToStaticMarkup(<Iteration stories={[andromeda, m1]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-iteration-9003"')
    expect(html).toContain('Iteration Andromeda')
    expect(html).toContain('(closed)')
    expect(html).toContain('(1)')
  })

  test('returns null when no iterations and no loose stories (existing behavior preserved)', () => {
    const html = renderToStaticMarkup(<Iteration stories={[]} onOpen={() => {}} />)
    expect(html).toBe('')
  })
})
