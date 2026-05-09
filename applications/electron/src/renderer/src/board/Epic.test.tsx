import { beforeEach, describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Epic from './Epic'
import { isActiveEpic, type BoardStory } from './types'

beforeEach(() => {
  if (typeof window !== 'undefined') {
    window.localStorage.clear()
  }
})

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

describe('#1185 Epic — meta-card DOM shape (falsifiers a-c, j)', () => {
  test('does NOT nest children inside the Epic card (falsifier-a)', () => {
    const epic = story({ number: 1100, title: 'parent-epic' })
    const child = story({
      number: 1101,
      title: 'child-feature',
      labels: featureLabels,
      parent_epic: 1100
    })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, child]} onOpen={() => {}} />
    )
    expect(html).not.toContain('child-feature')
    expect(html).not.toContain('board-epic-children')
    expect(html).not.toContain('board-epic-child-state')
  })

  test('renders epic description from User-job paragraph (falsifier-b)', () => {
    const body =
      '## User job\n\nAs the owner I want to see the Epic as a meta-card.\n\n## Observation falsifier\n\nReject if children nest under parent.\n'
    const epic = story({ number: 1100, body })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toContain('data-test="board-epic-1100-description"')
    expect(html).toContain('As the owner I want to see the Epic as a meta-card.')
    expect(html).not.toContain('Reject if children nest under parent')
  })

  test('falls back to first body paragraph when User-job heading is absent', () => {
    const body = 'Plain epic description without standard headings.\n\nMore text.'
    const epic = story({ number: 1100, body })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toContain('Plain epic description without standard headings.')
  })

  test('omits description block when body is empty', () => {
    const epic = story({ number: 1100, body: '' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).not.toContain('data-test="board-epic-1100-description"')
  })

  test('renders native <progress> element with value + max from children counts (falsifier-c, j)', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'in-progress', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'icebox', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1104, state: 'icebox', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1105, state: 'icebox', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1106, state: 'icebox', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('<progress')
    expect(html).toContain('value="1"')
    expect(html).toContain('max="6"')
    expect(html).toContain('data-test="board-epic-1100-progress"')
  })

  test('progress max=1 when no children (avoids zero-divide rendering)', () => {
    const epic = story({ number: 1100 })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toContain('value="0"')
    expect(html).toContain('max="1"')
  })

  test('progress aria-label exposes accepted/total + state-breakdown for screen readers', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'delivered', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toMatch(/aria-label="Progress: 1 of 2 children accepted/)
    expect(html).toContain('1 delivered')
    expect(html).toContain('0 in progress')
  })

  test('renders state-breakdown text with all 6 segments (falsifier-c)', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'delivered', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'in-progress', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1104, state: 'desk-check', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1105, state: 'ready', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1106, state: 'icebox', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('1/6 accepted')
    expect(html).toContain('1/6 delivered')
    expect(html).toContain('1/6 in-progress')
    expect(html).toContain('1/6 desk-check')
    expect(html).toContain('1/6 ready')
    expect(html).toContain('1/6 icebox')
  })

  test('breakdown rolls ready-for-accept into delivered + needs-fixes into in-progress + rejected into ready', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'ready-for-accept', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'needs-fixes', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'rejected', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('1/3 delivered')
    expect(html).toContain('1/3 in-progress')
    expect(html).toContain('1/3 ready')
  })

  test('progress re-derives from CURRENT children states on render (falsifier-d)', () => {
    const epic = story({ number: 1100 })
    const before: BoardStory[] = [
      story({ number: 1101, state: 'in-progress', labels: featureLabels, parent_epic: 1100 })
    ]
    const after: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 })
    ]
    const htmlBefore = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...before]} onOpen={() => {}} />
    )
    const htmlAfter = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...after]} onOpen={() => {}} />
    )
    expect(htmlBefore).toContain('value="0"')
    expect(htmlBefore).toContain('1/1 in-progress')
    expect(htmlAfter).toContain('value="1"')
    expect(htmlAfter).toContain('1/1 accepted')
  })

  test('Epic header is a button that opens drill-in via onOpen', () => {
    const epic = story({ number: 1100 })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toContain('class="board-epic-open-button"')
  })
})

describe('#1234 retired-Epic exclusion still applies through Epic surface', () => {
  test('renders only active Epics from a mixed stories array', () => {
    const stories: BoardStory[] = [
      story({ number: 1100, title: 'active-epic-A', state: 'in-progress' }),
      story({ number: 1184, title: 'retired-epic', state: 'icebox', gh_state: 'closed' }),
      story({ number: 1200, title: 'active-epic-B', state: 'ready' }),
      story({ number: 1300, title: 'accepted-epic', state: 'accepted', gh_state: 'closed' })
    ]
    const epics = stories.filter(isActiveEpic)
    const html = renderToStaticMarkup(
      <Epic epics={epics} stories={stories} onOpen={() => {}} />
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
      <Epic epics={epics} stories={stories} onOpen={() => {}} />
    )
    expect(html).toBe('')
  })
})

describe('#1234 isActiveEpic predicate (carried over from EpicSummary surface)', () => {
  test('open Epic in active workflow state → visible', () => {
    expect(isActiveEpic(story({ number: 100, state: 'in-progress' }))).toBe(true)
  })

  test('closed-Accepted Epic → visible (success-narrative preserved)', () => {
    expect(
      isActiveEpic(story({ number: 101, state: 'accepted', gh_state: 'closed' }))
    ).toBe(true)
  })

  test('retired Epic (icebox + gh_state=closed) → hidden', () => {
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
      expect(isActiveEpic(story({ number: 200, state, gh_state: 'closed' }))).toBe(false)
    }
  })

  test('non-Epic (Feature) → hidden regardless of state', () => {
    expect(isActiveEpic(story({ number: 300, labels: featureLabels }))).toBe(false)
  })
})

describe('#1235 Epic-card collapse — native <details>/<summary> + activity-class defaults + persistence', () => {
  test('Epic card wraps in native <details> + <summary> (falsifier-a)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toContain('<details')
    expect(html).toContain('<summary')
  })

  test('in-progress Epic is open by default (falsifier-c)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toMatch(/<details[^>]*\sopen\b/)
  })

  test('accepted Epic is closed by default (falsifier-b)', () => {
    const epic = story({ number: 1100, state: 'accepted' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
  })

  test('icebox Epic is closed by default (falsifier-b)', () => {
    const epic = story({ number: 1100, state: 'icebox' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
  })

  test('summary line shows compact accepted/total progress (falsifier-d)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'in-progress', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'icebox', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('data-test="board-epic-1100-summary-progress"')
    expect(html).toContain('1/3')
  })

  test('summary shows ✓ when all children accepted (100% complete)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'accepted', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('2/2 ✓')
  })

  test('summary omits ✓ when not 100% complete', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'in-progress', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('1/2')
    expect(html).not.toContain('1/2 ✓')
  })

  test('persisted-closed state in localStorage overrides default-open (falsifier-e)', () => {
    window.localStorage.setItem(
      'bpmux-epic-collapse:ryan',
      JSON.stringify({ '1100': false })
    )
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
  })

  test('persisted-open state in localStorage overrides default-closed', () => {
    window.localStorage.setItem(
      'bpmux-epic-collapse:ryan',
      JSON.stringify({ '1100': true })
    )
    const epic = story({ number: 1100, state: 'accepted' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toMatch(/<details[^>]*\sopen\b/)
  })

  test('summary still contains glyph + #N + state + title', () => {
    const epic = story({ number: 1100, state: 'in-progress', title: 'Epic Title Here' })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />
    )
    expect(html).toContain('#1100')
    expect(html).toContain('Epic Title Here')
    expect(html).toContain('data-test="board-epic-1100-state"')
  })

  test('expanded body contains description + progress + breakdown', () => {
    const epic = story({
      number: 1100,
      state: 'in-progress',
      body: '## User job\n\nDescription paragraph here.\n'
    })
    const child: BoardStory = story({
      number: 1101,
      state: 'accepted',
      labels: featureLabels,
      parent_epic: 1100
    })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, child]} onOpen={() => {}} />
    )
    expect(html).toContain('Description paragraph here.')
    expect(html).toContain('data-test="board-epic-1100-progress"')
    expect(html).toContain('data-test="board-epic-1100-breakdown"')
  })
})
