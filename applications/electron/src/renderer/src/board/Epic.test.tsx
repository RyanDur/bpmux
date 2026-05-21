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
  test('does NOT nest full child rows inside the Epic card (falsifier-a; mini-board badges are not full rows)', () => {
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
    expect(html).not.toContain('board-epic-children')
    expect(html).not.toContain('board-epic-child-state')
    expect(html).not.toContain('board-story-row')
  })

  test('renders epic description from User-job paragraph (falsifier-b)', () => {
    const body =
      '## User job\n\nAs the owner I want to see the Epic as a meta-card.\n\n## Observation falsifier\n\nReject if children nest under parent.\n'
    const epic = story({ number: 1100, body })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-epic-1100-description"')
    expect(html).toContain('As the owner I want to see the Epic as a meta-card.')
    expect(html).not.toContain('Reject if children nest under parent')
  })

  test('falls back to first body paragraph when User-job heading is absent', () => {
    const body = 'Plain epic description without standard headings.\n\nMore text.'
    const epic = story({ number: 1100, body })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('Plain epic description without standard headings.')
  })

  test('omits description block when body is empty', () => {
    const epic = story({ number: 1100, body: '' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
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
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('value="0"')
    expect(html).toContain('max="1"')
  })

  test('progress max excludes retired but INCLUDES icebox (canon-refinement 2026-05-20 23:12 UTC)', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'in-progress', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'icebox', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1104, state: 'icebox', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1105, state: 'retired', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('value="1"')
    expect(html).toContain('max="4"')
  })

  test('progress aria-label exposes accepted/denom + state-breakdown for screen readers (#1451 retire-aware)', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'delivered', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toMatch(/aria-label="Progress: 1 of 2 children needing completion accepted/)
    expect(html).toContain('1 delivered')
    expect(html).toContain('0 in progress')
    expect(html).toContain('0 retired')
  })

  test('progress aria-label rolls ready-for-accept into delivered + needs-fixes into in-progress + rejected into ready', () => {
    const epic = story({ number: 1100 })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'ready-for-accept', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'needs-fixes', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'rejected', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('1 delivered')
    expect(html).toContain('1 in progress')
    expect(html).toContain('1 ready')
  })

  test('progress + mini-board re-derive from CURRENT children states on render (falsifier-d)', () => {
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
    expect(htmlBefore).toMatch(/aria-label="In Progress: 1 child"/)
    expect(htmlAfter).toContain('value="1"')
    expect(htmlAfter).toMatch(/aria-label="Accepted: 1 child"/)
  })

  test('Epic header is a button that opens drill-in via onOpen', () => {
    const epic = story({ number: 1100 })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
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
    const html = renderToStaticMarkup(<Epic epics={epics} stories={stories} onOpen={() => {}} />)
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
    const html = renderToStaticMarkup(<Epic epics={epics} stories={stories} onOpen={() => {}} />)
    expect(html).toBe('')
  })
})

describe('#1234 isActiveEpic predicate (carried over from EpicSummary surface)', () => {
  test('open Epic in active workflow state → visible', () => {
    expect(isActiveEpic(story({ number: 100, state: 'in-progress' }))).toBe(true)
  })

  test('closed-Accepted Epic → visible (success-narrative preserved)', () => {
    expect(isActiveEpic(story({ number: 101, state: 'accepted', gh_state: 'closed' }))).toBe(true)
  })

  test('retired Epic (icebox + gh_state=closed) → hidden', () => {
    expect(isActiveEpic(story({ number: 1184, state: 'icebox', gh_state: 'closed' }))).toBe(false)
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
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('<details')
    expect(html).toContain('<summary')
  })

  test('in-progress Epic is open by default (falsifier-c)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const child = story({ number: 1101, state: 'in-progress', parent_epic: 1100 })
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, child]} onOpen={() => {}} />
    )
    expect(html).toMatch(/<details[^>]*\sopen\b/)
  })

  test('accepted Epic is closed by default (falsifier-b)', () => {
    const epic = story({ number: 1100, state: 'accepted' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
  })

  test('icebox Epic is closed by default (falsifier-b)', () => {
    const epic = story({ number: 1100, state: 'icebox' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
  })

  test('#1455 summary shows 3-axis count (accepted ✓ + active ●; retired hidden when 0)', () => {
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
    expect(html).toContain('data-test="board-epic-1100-count-accepted"')
    expect(html).toContain('1✓')
    expect(html).toContain('data-test="board-epic-1100-count-active"')
    expect(html).toContain('2●')
    expect(html).not.toContain('data-test="board-epic-1100-count-retired"')
  })

  test('#1455 summary shows ✓ marker when all children accepted (100% complete) + hides active when zero', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'accepted', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('2✓')
    expect(html).toContain('data-test="board-epic-1100-count-done"')
    expect(html).not.toContain('data-test="board-epic-1100-count-active"')
  })

  test('#1455 summary omits done-marker when not 100% complete', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'in-progress', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('1✓')
    expect(html).toContain('1●')
    expect(html).not.toContain('data-test="board-epic-1100-count-done"')
  })

  test('#1455 summary discriminates retired children when any present (Scenario 1+4 falsifier)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'retired', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1104, state: 'in-progress', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('data-test="board-epic-1100-count-accepted"')
    expect(html).toContain('2✓')
    expect(html).toContain('data-test="board-epic-1100-count-retired"')
    expect(html).toContain('1⊘')
    expect(html).toContain('data-test="board-epic-1100-count-active"')
    expect(html).toContain('1●')
  })

  test('#1455 Scenario 3 — title-prefix `[Epic]` retired from rendered title', () => {
    const epic = story({ number: 1100, title: '[Epic] Visual coherence', state: 'in-progress' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('Visual coherence')
    expect(html).not.toMatch(/\[Epic\]\s*Visual coherence/)
  })

  test('#1455 Scenario 8 — empty Epic (zero children) reads "empty" not "0/0"', () => {
    const epic = story({ number: 1100, title: 'Lonely Epic', state: 'icebox' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-epic-1100-count-empty"')
    expect(html).toContain('empty')
    expect(html).not.toContain('0/0')
  })

  test('persisted-closed state in localStorage overrides default-open (falsifier-e)', () => {
    window.localStorage.setItem('bpmux-epic-collapse:ryan', JSON.stringify({ '1100': false }))
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
  })

  test('persisted-open state in localStorage overrides default-closed', () => {
    window.localStorage.setItem('bpmux-epic-collapse:ryan', JSON.stringify({ '1100': true }))
    const epic = story({ number: 1100, state: 'accepted' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toMatch(/<details[^>]*\sopen\b/)
  })

  test('summary still contains glyph + #N + state + title', () => {
    const epic = story({ number: 1100, state: 'in-progress', title: 'Epic Title Here' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('#1100')
    expect(html).toContain('Epic Title Here')
    expect(html).toContain('data-test="board-epic-1100-state"')
  })

  test('expanded body contains description + mini-board (per #1244 replaces breakdown text)', () => {
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
    expect(html).toContain('data-test="board-epic-1100-miniboard"')
    expect(html).not.toContain('data-test="board-epic-1100-breakdown"')
  })

  test('#1243 progress bar renders inside <summary> for always-visible at all collapse-states', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    const summaryMatch = html.match(/<summary[^>]*>([\s\S]*?)<\/summary>/)
    expect(summaryMatch).not.toBeNull()
    expect(summaryMatch?.[1]).toContain('data-test="board-epic-1100-progress"')
  })

  test('#1243 progress bar carries thinner-summary class for CSS sizing', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('board-epic-progress--summary')
  })

  test('#1243 progress bar visible even when Epic <details> is closed by default', () => {
    const epic = story({ number: 1100, state: 'accepted' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).not.toMatch(/<details[^>]*\sopen\b/)
    const summaryMatch = html.match(/<summary[^>]*>([\s\S]*?)<\/summary>/)
    expect(summaryMatch?.[1]).toContain('<progress')
    expect(summaryMatch?.[1]).toContain('data-test="board-epic-1100-progress"')
  })
})

describe('#1244 Epic dropdown mini-board — replaces breakdown text', () => {
  test('renders mini-board container with all 6 named state-columns (falsifier-b)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-epic-1100-miniboard"')
    expect(html).toContain('data-test="board-epic-1100-miniboard-column-ready"')
    expect(html).toContain('data-test="board-epic-1100-miniboard-column-in-progress"')
    expect(html).toContain('data-test="board-epic-1100-miniboard-column-desk-check"')
    expect(html).toContain('data-test="board-epic-1100-miniboard-column-delivered"')
    expect(html).toContain('data-test="board-epic-1100-miniboard-column-accepted"')
    expect(html).toContain('data-test="board-epic-1100-miniboard-column-icebox"')
  })

  test('child appears in current state-column derived from canonical Story.state (falsifier-c)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({
        number: 1101,
        state: 'accepted',
        labels: featureLabels,
        parent_epic: 1100,
        title: 'accepted-child'
      }),
      story({
        number: 1102,
        state: 'in-progress',
        labels: featureLabels,
        parent_epic: 1100,
        title: 'in-progress-child'
      }),
      story({
        number: 1103,
        state: 'icebox',
        labels: featureLabels,
        parent_epic: 1100,
        title: 'icebox-child'
      })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const acceptedColMatch = html.match(
      /data-test="board-epic-1100-miniboard-column-accepted"[\s\S]*?<\/details>/
    )
    expect(acceptedColMatch?.[0]).toContain('#1101')
    expect(acceptedColMatch?.[0]).not.toContain('#1102')
    expect(acceptedColMatch?.[0]).not.toContain('#1103')

    const inProgressColMatch = html.match(
      /data-test="board-epic-1100-miniboard-column-in-progress"[\s\S]*?<\/details>/
    )
    expect(inProgressColMatch?.[0]).toContain('#1102')
    expect(inProgressColMatch?.[0]).not.toContain('#1101')
  })

  test('rolls ready-for-accept → delivered, needs-fixes → in-progress, rejected → ready (uses columnFor)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'ready-for-accept', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'needs-fixes', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'rejected', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const deliveredColMatch = html.match(
      /data-test="board-epic-1100-miniboard-column-delivered"[\s\S]*?<\/details>/
    )
    expect(deliveredColMatch?.[0]).toContain('#1101')

    const inProgressColMatch = html.match(
      /data-test="board-epic-1100-miniboard-column-in-progress"[\s\S]*?<\/details>/
    )
    expect(inProgressColMatch?.[0]).toContain('#1102')

    const readyColMatch = html.match(
      /data-test="board-epic-1100-miniboard-column-ready"[\s\S]*?<\/details>/
    )
    expect(readyColMatch?.[0]).toContain('#1103')
  })

  test('closed-non-accepted child appears in retired column (uses isClosed via gh_state)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({
        number: 1101,
        state: 'icebox',
        labels: featureLabels,
        parent_epic: 1100,
        gh_state: 'closed'
      })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const retiredColMatch = html.match(
      /data-test="board-epic-1100-miniboard-column-retired"[\s\S]*?<\/details>/
    )
    expect(retiredColMatch?.[0]).toContain('#1101')
  })

  test('each state-column is a native <details>/<summary> collapsible (falsifier-d, h native HTML5)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const acceptedColMatch = html.match(
      /<details[^>]*data-test="board-epic-1100-miniboard-column-accepted"[\s\S]*?<\/details>/
    )
    expect(acceptedColMatch).not.toBeNull()
    expect(acceptedColMatch?.[0]).toContain('<summary')
  })

  test('each child-badge surfaces #N + truncated title (falsifier-e)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({
        number: 1101,
        state: 'in-progress',
        labels: featureLabels,
        parent_epic: 1100,
        title: 'feature-with-truncatable-title'
      })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toContain('data-test="board-epic-1100-miniboard-child-1101"')
    expect(html).toContain('#1101')
    expect(html).toContain('feature-with-truncatable-title')
  })

  test('column shows count of children currently in that state', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'in-progress', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const acceptedColMatch = html.match(
      /<details[^>]*data-test="board-epic-1100-miniboard-column-accepted"[\s\S]*?<\/details>/
    )
    expect(acceptedColMatch?.[0]).toContain(
      'data-test="board-epic-1100-miniboard-column-accepted-count"'
    )
    expect(acceptedColMatch?.[0]).toMatch(/>\(?2\)?</)
  })

  test('description from extractDescription still renders alongside mini-board (falsifier-i regression)', () => {
    const epic = story({
      number: 1100,
      state: 'in-progress',
      body: '## User job\n\nMy description text.\n'
    })
    const html = renderToStaticMarkup(<Epic epics={[epic]} stories={[epic]} onOpen={() => {}} />)
    expect(html).toContain('data-test="board-epic-1100-description"')
    expect(html).toContain('My description text.')
    expect(html).toContain('data-test="board-epic-1100-miniboard"')
  })

  test('plain-text breakdown is replaced — no "X/Y accepted · ..." pattern in body (falsifier-a)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'delivered', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1103, state: 'icebox', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).not.toContain('1/3 accepted')
    expect(html).not.toContain('1/3 delivered')
    expect(html).not.toContain('1/3 icebox')
    expect(html).not.toContain('board-epic-breakdown')
  })

  test('column carries aria-label naming the state + count (falsifier-h ARIA)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 }),
      story({ number: 1102, state: 'accepted', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toMatch(/aria-label="Accepted: 2 children?"/)
  })

  test('child-badge carries aria-label naming story + state (falsifier-h ARIA)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({
        number: 1101,
        state: 'in-progress',
        labels: featureLabels,
        parent_epic: 1100,
        title: 'my-feature'
      })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    expect(html).toMatch(/aria-label="#1101 my-feature, in-progress"/)
  })

  test('children list inside column uses native <ul> (falsifier-h native HTML5 list semantics)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({ number: 1101, state: 'accepted', labels: featureLabels, parent_epic: 1100 })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const acceptedColMatch = html.match(
      /<details[^>]*data-test="board-epic-1100-miniboard-column-accepted"[\s\S]*?<\/details>/
    )
    expect(acceptedColMatch?.[0]).toContain('<ul')
    expect(acceptedColMatch?.[0]).toContain('<li')
  })

  test('non-Epic-children are excluded from mini-board (parent_epic match required)', () => {
    const epic = story({ number: 1100, state: 'in-progress' })
    const children: BoardStory[] = [
      story({
        number: 1101,
        state: 'accepted',
        labels: featureLabels,
        parent_epic: 1100,
        title: 'mine'
      }),
      story({ number: 9999, state: 'accepted', labels: featureLabels, title: 'not-mine' })
    ]
    const html = renderToStaticMarkup(
      <Epic epics={[epic]} stories={[epic, ...children]} onOpen={() => {}} />
    )
    const miniboardMatch = html.match(
      /data-test="board-epic-1100-miniboard"[\s\S]*?(?=data-test="board-epic-\d+-miniboard"|$)/
    )
    expect(miniboardMatch?.[0]).toContain('#1101')
    expect(miniboardMatch?.[0]).not.toContain('#9999')
  })
})
