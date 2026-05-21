import { storyTypeFromLabels, type BoardStory } from './types'

export type BadgeKind = 'team-accepted' | 'accepted' | 'rework' | 'blocked'

const DESK_CHECK_FAIL_LABEL = 'desk-check-fail'
const BLOCKED_LABEL = 'blocked'
const BLOCKED_BY_PREFIX = 'blocked-by:'
const ACCEPTOR_TEAM_LABEL = 'awaiting team'
const ACCEPTOR_OWNER_LABEL = 'awaiting owner'

export const blockerNumberFromLabels = (labels: string[]): number | null => {
  for (const l of labels) {
    const lower = l.toLowerCase()
    if (lower.startsWith(BLOCKED_BY_PREFIX)) {
      const n = Number.parseInt(lower.slice(BLOCKED_BY_PREFIX.length), 10)
      if (Number.isFinite(n) && n > 0) return n
    }
  }
  return null
}

export const deriveBadge = (story: BoardStory): BadgeKind | null => {
  if (story.state !== 'delivered' && story.state !== 'accepted') return null
  const normalized = story.labels.map((l) => l.toLowerCase())
  if (story.state === 'delivered' && normalized.includes(BLOCKED_LABEL)) {
    return 'blocked'
  }
  if (normalized.includes(DESK_CHECK_FAIL_LABEL)) return 'rework'
  if (normalized.includes(ACCEPTOR_TEAM_LABEL)) return 'team-accepted'
  if (normalized.includes(ACCEPTOR_OWNER_LABEL)) return 'accepted'
  const type = storyTypeFromLabels(story.labels)
  if (type === 'bug' || type === 'chore') return 'team-accepted'
  return 'accepted'
}

export const labelFor = (kind: BadgeKind, state: BoardStory['state']): string => {
  if (kind === 'blocked') return 'blocked'
  if (kind === 'rework') return 'rework'
  if (state === 'accepted') {
    return kind === 'team-accepted' ? 'team-accepted' : 'owner-accepted'
  }
  return kind === 'team-accepted' ? 'awaiting team' : 'awaiting owner'
}
