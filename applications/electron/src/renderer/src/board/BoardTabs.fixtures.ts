import type BoardTabs from './BoardTabs'
import {
  GROUP_ORDER,
  groupStoriesIn,
  middleColumnIds,
  type Epic,
  type GroupSpec,
  type Story,
  type ColumnState
} from './types'
import { selectIterations } from './selectIterations'
import { getSessionSnapshot } from '../transport/session/store'

export const iterationStory = (overrides: Partial<Story>): Story => ({
  id: 9000,
  title: 'Iteration Sextans',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration', 'iteration-order.100'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

export const epicStory = (overrides: Partial<Story>): Story => ({
  id: 8000,
  title: 'Epic Foo',
  state: 'in-progress',
  gh_state: 'open',
  labels: ['epic'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

export const featureStory = (overrides: Partial<Story>): Story => ({
  id: 9100,
  title: 'a feature',
  state: 'ready',
  gh_state: 'open',
  labels: ['feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

export const buildProps = (
  stories: Story[],
  groupOrder: GroupSpec[] = GROUP_ORDER
): Parameters<typeof BoardTabs>[0] => {
  const epics: Epic[] = stories
    .filter((s) => s.labels.includes('epic'))
    .map((s) => ({
      id: s.id,
      title: s.title,
      state: s.state,
      closed: s.gh_state === 'closed',
      created_at: s.created_at,
      body: s.body
    }))
  const epicTitleById = new Map(epics.map((e) => [e.id, e.title]))
  const iterationContainerStories = stories.filter(
    (s) => s.labels.includes('iteration') || s.state === 'icebox'
  )
  const flat = stories.filter((s) => !s.labels.includes('epic') && s.state !== 'icebox')
  const grouped = groupStoriesIn(middleColumnIds(groupOrder), flat)
  return {
    stories,
    iterations: selectIterations(getSessionSnapshot()),
    groupOrder,
    epics,
    epicTitleById,
    iterationContainerStories,
    grouped,
    loading: false,
    effectiveSelected: null,
    collapsed: {
      icebox: false,
      ready: false,
      'in-progress': false,
      'desk-check': false,
      acceptance: false,
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
