import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import EpicSummary from './EpicSummary'
import { isActiveEpic, type BoardStory } from './types'

// #1234: retired Epics (closed-non-Accepted, e.g. moved to Icebox +
// closed for over-Epic'd reshape) must NOT appear in the renderer's
// active Epic-summary list. Sister to columnFor's retired-bucket
// pattern (#514) and to #1230/#1231 stale-state class. The fix lives
// at Board.tsx where `epics` is derived; the predicate is hoisted to
// `isActiveEpic` in types.ts so this falsifier can pin it without
// mounting Board's WS lifecycle.

const epicLabels = ['Epic']
const featureLabels = ['Feature']

const story = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 1,
  title: 'stub',
  state: 'ready',
  labels: epicLabels,
  assignees: [],
  created_at: '2026-05-09T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1234 isActiveEpic — Epic-summary visibility predicate', () => {
  test('open Epic in active workflow state → visible', () => {
    expect(isActiveEpic(story({ number: 100, state: 'in-progress' }))).toBe(true)
  })

  test('closed-Accepted Epic → visible (success-narrative preserved)', () => {
    // mirrors columnFor's `accepted` carveout: an Epic whose children
    // all accepted should still show as a meta-card so the team sees
    // the full progress bar at-a-glance. Retire-vs-Accept is the
    // distinguishing axis, not closed-state.
    expect(
      isActiveEpic(story({ number: 101, state: 'accepted', gh_state: 'closed' }))
    ).toBe(true)
  })

  test('retired Epic (icebox + gh_state=closed) → hidden — load-bearing for #1234', () => {
    expect(
      isActiveEpic(story({ number: 1184, state: 'icebox', gh_state: 'closed' }))
    ).toBe(false)
  })

  test('retired Epic from any non-Accepted state → hidden', () => {
    const states: BoardStory['state'][] = [
      'icebox',
      'ready',
      'in-progress',
      'desk-check',
      'delivered',
      'ready-for-accept',
      'needs-fixes',
      'rejected'
    ]
    for (const state of states) {
      expect(
        isActiveEpic(story({ number: 200, state, gh_state: 'closed' }))
      ).toBe(false)
    }
  })

  test('non-Epic (Feature) → hidden regardless of state', () => {
    expect(isActiveEpic(story({ number: 300, labels: featureLabels }))).toBe(false)
  })
})

describe('#1234 EpicSummary — retired-Epic exclusion at the rendered surface', () => {
  // Integration falsifier: pipes a mixed stories array through
  // isActiveEpic (mirroring Board.tsx:231) and hands the filtered
  // result to EpicSummary. Asserts the rendered DOM shape contains
  // the active Epics and excludes the retired one. Pins the
  // predicate-to-renderer wiring without mounting Board.
  test('renders only active Epics from a mixed stories array', () => {
    const stories: BoardStory[] = [
      story({ number: 1100, title: 'active-epic-A', state: 'in-progress' }),
      story({
        number: 1184,
        title: 'retired-epic',
        state: 'icebox',
        gh_state: 'closed'
      }),
      story({ number: 1200, title: 'active-epic-B', state: 'ready' }),
      story({
        number: 1300,
        title: 'accepted-epic',
        state: 'accepted',
        gh_state: 'closed'
      })
    ]
    const epics = stories.filter(isActiveEpic)
    const html = renderToStaticMarkup(
      <EpicSummary epics={epics} stories={stories} onOpen={() => {}} />
    )

    expect(html).toContain('data-test="board-epic-1100"')
    expect(html).toContain('data-test="board-epic-1200"')
    expect(html).toContain('data-test="board-epic-1300"')
    expect(html).not.toContain('data-test="board-epic-1184"')
  })

  test('returns null when every Epic in the source list is retired', () => {
    const stories: BoardStory[] = [
      story({ number: 1184, state: 'icebox', gh_state: 'closed' }),
      story({ number: 1185, state: 'ready', gh_state: 'closed' })
    ]
    const epics = stories.filter(isActiveEpic)
    const html = renderToStaticMarkup(
      <EpicSummary epics={epics} stories={stories} onOpen={() => {}} />
    )
    expect(html).toBe('')
  })
})
