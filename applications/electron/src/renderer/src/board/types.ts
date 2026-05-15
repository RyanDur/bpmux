// Wire-protocol types mirroring `GET /api/board` +
// `GET /api/stories/:number` from the Rust backend.
//
// Keep in sync with applications/bpmux/src/http/board.rs. Any field
// added server-side should flow through here before it hits the
// components.

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

export type ColumnState =
  | 'icebox'
  | 'ready'
  | 'in-progress'
  | 'desk-check'
  | 'delivered'
  | 'accepted'
  | 'retired'

// Closed-non-Accepted stories project to RETIRED (close_story for
// retire / dedupe / won't-fix); closed-Accepted stays in ACCEPTED so
// the success-narrative isn't polluted by decision-not-to-pursue
// closures. #514 separates substrate (state-decoder) from visual
// (column-bucketing): #464 owns "what state was this last in"; this
// function owns "which active-work bucket should the user see this
// story in right now."
export const columnFor = (state: StoryState, isClosed: boolean = false): ColumnState => {
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

export type StoryType =
  | 'feature'
  | 'bug'
  | 'chore'
  | 'epic'
  | 'iteration'
  | 'unknown'

export interface BoardStory {
  number: number
  title: string
  state: StoryState
  // GitHub issue state: "open" or "closed". Distinct from `state`,
  // which is the bpmux workflow-state derived per #464 (preserves
  // last domain state on closed-without-explicit-accept). columnFor
  // reads this to keep closed stories out of active-work columns
  // regardless of preserved last-state (#514).
  gh_state?: 'open' | 'closed'
  labels: string[]
  assignees: string[]
  created_at: string
  body: string
  parent_epic?: number
  iteration_id?: number
  // #1146: server-derived rework-context. When the most-recent state
  // transition is backward (reject_story Delivered→Ready or
  // fail_desk_check DeskCheck→InProgress), the gate-comment's payload
  // surfaces here so the renderer can show ↩ glyph + tooltip without
  // a per-story round-trip. Null when no backward edge or when a
  // later forward transition has superseded.
  rework_comment?: string | null
}

export type IceboxItem =
  | { kind: 'story'; number: number }
  | { kind: 'iteration'; number: number }

export interface BoardResponse {
  stories: BoardStory[]
  /** #1258 α-substrate: unified ordering for the icebox column.
   * Markers + stories share one positional space. When empty (e.g.
   * pre-α legacy state), renderer falls back to markers-then-stories
   * concatenation. */
  icebox_unified?: IceboxItem[]
  fetched_at: string
}

// Display order for state groups + human-readable label.
export interface GroupSpec {
  state: ColumnState
  label: string
  // Collapsed by default (still visible, opened on click). `<details>`
  // needs the open attribute to default to expanded, so defaultOpen =
  // !collapsed.
  collapsed: boolean
}

// AWAITING-ACCEPT removed (#359): phantom column — no backend label
// distinguishes "delivered awaiting team accept" from plain "delivered"
// yet. Reserving the column pre-populated user expectation as a bug
// when it stayed empty in every session. Reintroduce when phase-2's
// team-accept-badge (held story #D) lands with real semantics; at
// that point the column can be renamed based on observed behavior,
// not pre-reserved vocabulary.
export const GROUP_ORDER: GroupSpec[] = [
  { state: 'ready', label: 'Ready', collapsed: false },
  { state: 'in-progress', label: 'In Progress', collapsed: false },
  { state: 'desk-check', label: 'Desk Check', collapsed: false },
  { state: 'delivered', label: 'Delivered', collapsed: false },
  { state: 'accepted', label: 'Accepted', collapsed: true },
  { state: 'retired', label: 'Retired', collapsed: true },
  { state: 'icebox', label: 'Icebox', collapsed: true }
]

// Inverse mapping is partial — closed-non-Accepted projects to Retired
// from any non-Accepted StoryState, so the Retired column is keyed on
// is_closed rather than on a specific state. Listed empty here to keep
// the Record exhaustive; columnFor is the source of truth.
export const STATES_IN_COLUMN: Record<ColumnState, StoryState[]> = {
  icebox: ['icebox'],
  ready: ['ready', 'rejected'],
  'in-progress': ['in-progress', 'needs-fixes'],
  'desk-check': ['desk-check'],
  delivered: ['delivered', 'ready-for-accept'],
  accepted: ['accepted'],
  retired: []
}

// Derive story type from labels (case-insensitive). Defaults to
// 'unknown' when no type label is present — keeps the row glyph
// predictable instead of guessing.
export const storyTypeFromLabels = (labels: string[]): StoryType => {
  const normalized = labels.map((l) => l.toLowerCase())
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

// #1302 rework: sister to isActiveEpic — iteration-issues with
// story_type=Iteration that are not retired (closed-without-accept)
// show as parent-rows in the icebox section. Closed iterations
// preserve in retired view per #1278 historical-artifact-visibility.
export const isActiveIteration = (story: BoardStory): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'iteration') return false
  return columnFor(story.state, story.gh_state === 'closed') !== 'retired'
}

export const isClosedIterationMarker = (story: BoardStory): boolean => {
  if (storyTypeFromLabels(story.labels) !== 'iteration') return false
  return story.gh_state === 'closed'
}

export const deriveEpicState = (
  epicExplicitState: StoryState,
  children: BoardStory[]
): StoryState => {
  if (epicExplicitState === 'accepted') return 'accepted'
  if (children.length === 0) return epicExplicitState
  if (children.some((c) => c.state === 'in-progress')) return 'in-progress'
  if (children.every((c) => c.state === 'delivered' || c.state === 'accepted')) {
    return 'delivered'
  }
  return epicExplicitState
}

export const groupStories = (stories: BoardStory[]): Map<ColumnState, BoardStory[]> => {
  const out = new Map<ColumnState, BoardStory[]>()
  for (const group of GROUP_ORDER) out.set(group.state, [])
  for (const story of stories) {
    const list = out.get(columnFor(story.state, story.gh_state === 'closed'))
    if (list) list.push(story)
  }
  return out
}
