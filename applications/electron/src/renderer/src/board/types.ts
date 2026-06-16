export type StoryState =
  | 'icebox'
  | 'ready'
  | 'in-progress'
  | 'desk-check'
  | 'delivered'
  | 'ready-for-accept'
  | 'needs-fixes'
  | 'rejected'
  | 'accepted'
  | 'retired'

export type ColumnState =
  | 'icebox'
  | 'ready'
  | 'in-progress'
  | 'desk-check'
  | 'delivered'
  | 'accepted'
  | 'retired'

export const columnFor = (
  state: StoryState,
  isClosed: boolean = false,
  storyType: StoryType = 'unknown'
): ColumnState => {
  if (isClosed && storyType === 'iteration') {
    return 'icebox'
  }
  if (isClosed && state !== 'accepted') {
    return 'retired'
  }
  switch (state) {
    case 'ready-for-accept':
      return 'delivered'
    case 'needs-fixes':
      return 'in-progress'
    case 'rejected':
      return 'ready'
    default:
      return state
  }
}

export type StoryType = 'feature' | 'bug' | 'chore' | 'epic' | 'iteration' | 'outcome' | 'unknown'

export type Acceptor = 'team' | 'owner' | 'dev'

export type Horizon = 'now' | 'next' | 'later'

export interface IterationMembership {
  iteration_number: number
  position: number
}

export interface OutcomeReadiness {
  iteration_count: number
  gap_count: number
  all_ready: boolean
}

export interface IterationReadiness {
  drafted: boolean
  stories_ordered: boolean
  review_closed: boolean
  dev_ok: boolean
}

export interface OutcomeMembership {
  outcome_number: number
  position: number
}

export interface BoardStory {
  number: number
  title: string
  state: StoryState
  gh_state?: 'open' | 'closed'
  labels: string[]
  assignees: string[]
  created_at: string
  body: string
  parent_epic?: number
  iteration_membership?: IterationMembership | null
  iteration_order?: number | null
  is_current?: boolean
  acceptor?: Acceptor | null
  rework_comment?: string | null
  horizon?: Horizon | null
  outcome_readiness?: OutcomeReadiness | null
  outcome_membership?: OutcomeMembership | null
  readiness?: IterationReadiness | null
}

export interface BoardResponse {
  stories: BoardStory[]
  fetched_at: string
}

export interface GroupSpec {
  state: ColumnState
  label: string
  collapsed: boolean
}

export const GROUP_ORDER: GroupSpec[] = [
  { state: 'ready', label: 'Ready', collapsed: false },
  { state: 'in-progress', label: 'In Progress', collapsed: false },
  { state: 'desk-check', label: 'Desk Check', collapsed: false },
  { state: 'delivered', label: 'Acceptance', collapsed: false },
  { state: 'accepted', label: 'Accepted', collapsed: true },
  { state: 'retired', label: 'Retired', collapsed: true },
  { state: 'icebox', label: 'Icebox', collapsed: false }
]

export const STATES_IN_COLUMN: Record<ColumnState, StoryState[]> = {
  icebox: ['icebox'],
  ready: ['ready', 'rejected'],
  'in-progress': ['in-progress', 'needs-fixes'],
  'desk-check': ['desk-check'],
  delivered: ['delivered', 'ready-for-accept'],
  accepted: ['accepted'],
  retired: []
}

export const storyTypeFromLabels = (labels: string[]): StoryType => {
  const normalized = labels.map((l) => l.toLowerCase())
  if (normalized.includes('outcome')) return 'outcome'
  if (normalized.includes('iteration')) return 'iteration'
  if (normalized.includes('epic')) return 'epic'
  if (normalized.includes('feature')) return 'feature'
  if (normalized.includes('bug')) return 'bug'
  if (normalized.includes('chore')) return 'chore'
  return 'unknown'
}

export const isActiveEpic = (story: BoardStory): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'epic') return false
  return columnFor(story.state, story.gh_state === 'closed') !== 'retired'
}

export const isActiveIteration = (story: BoardStory): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'iteration') return false
  return columnFor(story.state, story.gh_state === 'closed') !== 'retired'
}

export const isClosedIterationMarker = (story: BoardStory): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'iteration') return false
  return story.gh_state === 'closed'
}

export const deriveEpicState = (
  _epicExplicitState: StoryState,
  children: BoardStory[]
): StoryState => {
  if (children.length === 0 || children.every((c) => c.state === 'icebox')) {
    return 'icebox'
  }
  const anyActive = children.some(
    (c) =>
      c.state === 'in-progress' ||
      c.state === 'desk-check' ||
      c.state === 'delivered' ||
      c.state === 'ready-for-accept'
  )
  if (anyActive) return 'in-progress'
  const nonRetired = children.filter((c) => c.state !== 'retired')
  if (nonRetired.length === 0) return 'retired'
  if (nonRetired.every((c) => c.state === 'accepted')) return 'accepted'
  if (children.some((c) => c.state === 'ready')) return 'ready'
  return 'icebox'
}

export const groupStories = (stories: BoardStory[]): Map<ColumnState, BoardStory[]> => {
  const out = new Map<ColumnState, BoardStory[]>()
  for (const group of GROUP_ORDER) out.set(group.state, [])
  for (const story of stories) {
    const list = out.get(
      columnFor(story.state, story.gh_state === 'closed', storyTypeFromLabels(story.labels))
    )
    if (list) list.push(story)
  }
  return out
}
