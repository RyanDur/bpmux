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
  | 'delivered'
  | 'awaiting-accept'
  | 'accepted'

export type StoryType = 'feature' | 'bug' | 'chore' | 'unknown'

export interface BoardStory {
  number: number
  title: string
  state: StoryState
  labels: string[]
  assignees: string[]
  created_at: string
  body: string
}

export interface BoardResponse {
  stories: BoardStory[]
  fetched_at: string
}

// Display order for state groups + human-readable label.
export interface GroupSpec {
  state: StoryState
  label: string
  // Collapsed by default (still visible, opened on click). `<details>`
  // needs the open attribute to default to expanded, so defaultOpen =
  // !collapsed.
  collapsed: boolean
}

export const GROUP_ORDER: GroupSpec[] = [
  { state: 'ready', label: 'Ready', collapsed: false },
  { state: 'in-progress', label: 'In Progress', collapsed: false },
  { state: 'delivered', label: 'Delivered', collapsed: false },
  { state: 'awaiting-accept', label: 'Awaiting Accept', collapsed: false },
  { state: 'accepted', label: 'Accepted', collapsed: true },
  { state: 'icebox', label: 'Icebox', collapsed: true }
]

// Derive story type from labels (case-insensitive). Defaults to
// 'unknown' when no type label is present — keeps the row glyph
// predictable instead of guessing.
export function storyTypeFromLabels(labels: string[]): StoryType {
  const normalized = labels.map((l) => l.toLowerCase())
  if (normalized.includes('feature')) return 'feature'
  if (normalized.includes('bug')) return 'bug'
  if (normalized.includes('chore')) return 'chore'
  return 'unknown'
}
