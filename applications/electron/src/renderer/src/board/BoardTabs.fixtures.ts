import type BoardTabs from './BoardTabs'
import { groupStories, type BoardStory, type ColumnState } from './types'

export const iterationStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9000,
  title: 'Iteration Sextans',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration', 'iteration-order.100'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

export const epicStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 8000,
  title: 'Epic Foo',
  state: 'in-progress',
  gh_state: 'open',
  labels: ['epic'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

export const featureStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9100,
  title: 'a feature',
  state: 'ready',
  gh_state: 'open',
  labels: ['feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

export const buildProps = (stories: BoardStory[]): Parameters<typeof BoardTabs>[0] => {
  const epics = stories.filter((s) => s.labels.includes('epic'))
  const epicTitleByNumber = new Map(epics.map((e) => [e.number, e.title]))
  const iterationContainerStories = stories.filter(
    (s) => s.labels.includes('iteration') || s.state === 'icebox'
  )
  const flat = stories.filter((s) => !s.labels.includes('epic') && s.state !== 'icebox')
  const grouped = groupStories(flat)
  return {
    stories,
    epics,
    epicTitleByNumber,
    iterationContainerStories,
    grouped,
    loading: false,
    effectiveSelected: null,
    collapsed: {
      icebox: false,
      ready: false,
      'in-progress': false,
      'desk-check': false,
      delivered: false,
      accepted: true,
      retired: true
    } as Record<ColumnState, boolean>,
    onToggleCollapse: () => {},
    onOpen: () => {},
    onOpenTimeline: () => {},
    onSelect: () => {},
    onReorder: () => {},
    onReorderAnnounce: () => {}
  }
}
