export type StoryState = string

export type ColumnState = string

export interface BoardColumn {
  id: string
  label: string
}

export type StoryType = 'feature' | 'bug' | 'chore' | 'epic' | 'iteration' | 'outcome' | 'unknown'

export type Acceptor = 'team' | 'owner' | 'dev'

export type Horizon = 'now' | 'next' | 'later'

export const ICEBOX_COLUMN: ColumnState = 'icebox'
export const READY_COLUMN: ColumnState = 'ready'
export const ACCEPTANCE_COLUMN: ColumnState = 'acceptance'
export const ACCEPTED_COLUMN: ColumnState = 'accepted'
export const RETIRED_COLUMN: ColumnState = 'retired'

const FIXED_COLUMN_IDS = new Set<string>([
  ICEBOX_COLUMN,
  READY_COLUMN,
  ACCEPTANCE_COLUMN,
  ACCEPTED_COLUMN,
  RETIRED_COLUMN
])

const COLLAPSED_COLUMN_IDS = new Set<string>([ACCEPTED_COLUMN, RETIRED_COLUMN])

const DEFAULT_MIDDLE_IDS: readonly string[] = ['in-progress', 'desk-check']

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

export const columnForIn =
  (middleIds: readonly string[]) =>
  (state: StoryState, isClosed: boolean = false, storyType: StoryType = 'unknown'): ColumnState => {
    if (isClosed && storyType === 'iteration') {
      return ICEBOX_COLUMN
    }
    if (isClosed && state !== ACCEPTED_COLUMN) {
      return RETIRED_COLUMN
    }
    const firstMiddle = middleIds[0] ?? DEFAULT_MIDDLE_IDS[0]
    switch (state) {
      case 'ready-for-accept':
        return ACCEPTANCE_COLUMN
      case 'rejected':
        return READY_COLUMN
      case 'needs-fixes':
        return firstMiddle
    }
    if (middleIds.includes(state) || FIXED_COLUMN_IDS.has(state)) {
      return state
    }
    return firstMiddle
  }

export const columnFor = columnForIn(DEFAULT_MIDDLE_IDS)

export interface IterationMembership {
  iteration_id: number
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
  outcome_id: number
  position: number
}

export interface Story {
  id: number
  title: string
  state: StoryState
  gh_state?: 'open' | 'closed'
  labels: string[]
  assignees: string[]
  created_at: string
  body: string
  parent_epic?: number | null
  iteration_order?: number | null
  is_current?: boolean
  acceptor?: Acceptor | null
  rework_comment?: string | null
  horizon?: Horizon | null
  outcome_readiness?: OutcomeReadiness | null
  outcome_membership?: OutcomeMembership | null
  readiness?: IterationReadiness | null
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
  { state: 'acceptance', label: 'Acceptance', collapsed: false },
  { state: 'accepted', label: 'Accepted', collapsed: true },
  { state: 'retired', label: 'Retired', collapsed: true },
  { state: 'icebox', label: 'Icebox', collapsed: false }
]

export const middleColumnIds = (order: GroupSpec[]): string[] =>
  order.filter((group) => !FIXED_COLUMN_IDS.has(group.state)).map((group) => group.state)

export const groupOrderFrom = (columns: readonly BoardColumn[]): GroupSpec[] => {
  const toSpec = (column: BoardColumn): GroupSpec => ({
    state: column.id,
    label: column.label,
    collapsed: COLLAPSED_COLUMN_IDS.has(column.id)
  })
  const find = (id: string): BoardColumn | undefined => columns.find((c) => c.id === id)
  const middle = columns.filter((c) => !FIXED_COLUMN_IDS.has(c.id))
  const ordered: BoardColumn[] = []
  const ready = find(READY_COLUMN)
  if (ready) ordered.push(ready)
  ordered.push(...middle)
  const acceptance = find(ACCEPTANCE_COLUMN)
  if (acceptance) ordered.push(acceptance)
  const accepted = find(ACCEPTED_COLUMN)
  if (accepted) ordered.push(accepted)
  const retired = find(RETIRED_COLUMN)
  if (retired) ordered.push(retired)
  const icebox = find(ICEBOX_COLUMN)
  if (icebox) ordered.push(icebox)
  return ordered.map(toSpec)
}

export const groupOrderOrFallback = (columns: readonly BoardColumn[]): GroupSpec[] =>
  columns.length > 0 ? groupOrderFrom(columns) : GROUP_ORDER

export const isActiveEpic = (story: Story): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'epic') return false
  return columnFor(story.state, story.gh_state === 'closed') !== RETIRED_COLUMN
}

export const isActiveIteration = (story: Story): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'iteration') return false
  return columnFor(story.state, story.gh_state === 'closed') !== RETIRED_COLUMN
}

export const isClosedIterationMarker = (story: Story): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'iteration') return false
  return story.gh_state === 'closed'
}

export const deriveEpicState = (_epicExplicitState: StoryState, children: Story[]): StoryState => {
  if (children.length === 0 || children.every((c) => c.state === 'icebox')) {
    return 'icebox'
  }
  const anyActive = children.some(
    (c) =>
      c.state === 'in-progress' ||
      c.state === 'desk-check' ||
      c.state === 'acceptance' ||
      c.state === 'ready-for-accept'
  )
  if (anyActive) return 'in-progress'
  const nonRetired = children.filter((c) => c.state !== 'retired')
  if (nonRetired.length === 0) return 'retired'
  if (nonRetired.every((c) => c.state === 'accepted')) return 'accepted'
  if (children.some((c) => c.state === 'ready')) return 'ready'
  return 'icebox'
}

export const groupStoriesIn = (
  middleIds: readonly string[],
  stories: Story[]
): Map<ColumnState, Story[]> => {
  const bucketOf = columnForIn(middleIds)
  const out = new Map<ColumnState, Story[]>()
  const seedIds = [
    READY_COLUMN,
    ...middleIds,
    ACCEPTANCE_COLUMN,
    ACCEPTED_COLUMN,
    RETIRED_COLUMN,
    ICEBOX_COLUMN
  ]
  for (const id of seedIds) out.set(id, [])
  for (const story of stories) {
    const list = out.get(
      bucketOf(story.state, story.gh_state === 'closed', storyTypeFromLabels(story.labels))
    )
    if (list) list.push(story)
  }
  return out
}
