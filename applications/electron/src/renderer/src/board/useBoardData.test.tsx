import { describe, expect, test } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardData } from './useBoardData'
import type { BoardIterationData } from './iteration/selectIterations'
import { GROUP_ORDER, type BoardColumn, type Epic, type Story } from './types'
const story = (overrides: Partial<Story>): Story => ({
  id: 0,
  title: 'story',
  state: 'icebox',
  labels: [],
  assignees: [],
  created_at: '2026-05-24T00:00:00Z',
  body: '',
  ...overrides
})

const anEpic = (overrides: Partial<Epic>): Epic => ({
  id: 0,
  title: 'epic',
  state: 'in-progress',
  closed: false,
  created_at: '2026-05-24T00:00:00Z',
  body: '',
  ...overrides
})

const iteration = (id: number, stories: number[]): BoardIterationData => ({
  id,
  title: `Iteration ${id}`,
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories
})

const containerIds = (stories: Story[], iterations: BoardIterationData[] = []): number[] => {
  const { result } = renderHook(() => useBoardData(stories, iterations))
  return result.current.iterationContainerStories.map((s) => s.id)
}

describe('facet 1 — iterationContainerStories gates on membership, not workflow-state', () => {
  test('a canonical member promoted to ready stays in the iteration container (empty-Crux fix)', () => {
    const member = story({ id: 1547, state: 'ready', labels: ['ready', 'bug'] })
    expect(containerIds([member], [iteration(1543, [1547])])).toContain(1547)
  })

  test('a canonical member is included in every active workflow-state', () => {
    const states = ['ready', 'in-progress', 'desk-check', 'acceptance'] as const
    const members = states.map((state, i) =>
      story({ id: 2000 + i, state, labels: [state, 'feature'] })
    )
    const iter = iteration(
      1543,
      members.map((m) => m.id)
    )
    const got = containerIds(members, [iter])
    for (const m of members) expect(got).toContain(m.id)
  })

  test('a loose icebox story (no membership) still appears in the container', () => {
    const loose = story({ id: 900, state: 'icebox', labels: ['chore'] })
    expect(containerIds([loose])).toContain(900)
  })

  test('a ready story that is neither an iteration nor a member is NOT in the container', () => {
    const looseReady = story({ id: 901, state: 'ready', labels: ['ready', 'chore'] })
    expect(containerIds([looseReady])).not.toContain(901)
  })
})

describe('epics are sourced from the wire param, not derived from the story stream', () => {
  test('active epics from the wire param surface as epics, retired ones are excluded', () => {
    const active = anEpic({ id: 8000, state: 'in-progress' })
    const retired = anEpic({ id: 8001, state: 'icebox', closed: true })
    const { result } = renderHook(() => useBoardData([], [], [], [active, retired]))
    expect(result.current.epics.map((e) => e.id)).toEqual([8000])
  })

  test('an epic-labelled story present in the story stream is NOT promoted to an epic', () => {
    const strayEpic = story({ id: 8000, state: 'in-progress', labels: ['epic'] })
    const { result } = renderHook(() => useBoardData([strayEpic], [], [], []))
    expect(result.current.epics).toEqual([])
  })

  test('epicTitleById is built from the wire epics, including titles for retired epics', () => {
    const active = anEpic({ id: 8000, title: 'Active epic' })
    const retired = anEpic({ id: 8001, title: 'Retired epic', closed: true })
    const { result } = renderHook(() => useBoardData([], [], [], [active, retired]))
    expect(result.current.epicTitleById.get(8000)).toBe('Active epic')
    expect(result.current.epicTitleById.get(8001)).toBe('Retired epic')
  })
})

describe('useBoardData — groupOrder + bucketing follow the wire columns', () => {
  const threeStageColumns: BoardColumn[] = [
    { id: 'icebox', label: 'Icebox' },
    { id: 'ready', label: 'Ready' },
    { id: 'build', label: 'Build' },
    { id: 'review', label: 'Review' },
    { id: 'harden', label: 'Harden' },
    { id: 'acceptance', label: 'Acceptance' },
    { id: 'accepted', label: 'Accepted' },
    { id: 'retired', label: 'Retired' }
  ]

  test('with no wire columns, groupOrder falls back to the legacy GROUP_ORDER', () => {
    const { result } = renderHook(() => useBoardData([], []))
    expect(result.current.groupOrder).toBe(GROUP_ORDER)
  })

  test('with wire columns, groupOrder is the configured display order', () => {
    const { result } = renderHook(() => useBoardData([], [], [], [], threeStageColumns))
    expect(result.current.groupOrder.map((g) => g.state)).toEqual([
      'ready',
      'build',
      'review',
      'harden',
      'acceptance',
      'accepted',
      'retired',
      'icebox'
    ])
  })

  test('a story in a configured middle stage buckets into that stage column', () => {
    const member = story({ id: 9300, state: 'review', labels: ['feature'] })
    const { result } = renderHook(() => useBoardData([member], [], [], [], threeStageColumns))
    expect(result.current.grouped.get('review')?.map((s) => s.id)).toEqual([9300])
  })
})
