import { type Story } from './types'

export type BadgeKind =
  | 'team-accepted'
  | 'accepted'
  | 'dev-accepted'
  | 'pending'
  | 'rework'
  | 'blocked'

const DESK_CHECK_FAIL_LABEL = 'desk-check-fail'
const BLOCKED_LABEL = 'blocked'
const BLOCKED_BY_PREFIX = 'blocked-by:'

export const blockerIdFromLabels = (labels: string[]): number | null => {
  for (const l of labels) {
    const lower = l.toLowerCase()
    if (lower.startsWith(BLOCKED_BY_PREFIX)) {
      const n = Number.parseInt(lower.slice(BLOCKED_BY_PREFIX.length), 10)
      if (Number.isFinite(n) && n > 0) return n
    }
  }
  return null
}

export const deriveBadge = (story: Story): BadgeKind | null => {
  if (story.state !== 'delivered' && story.state !== 'accepted') return null
  const normalized = story.labels.map((l) => l.toLowerCase())
  if (story.state === 'delivered' && normalized.includes(BLOCKED_LABEL)) {
    return 'blocked'
  }
  if (normalized.includes(DESK_CHECK_FAIL_LABEL)) return 'rework'
  switch (story.acceptor) {
    case 'team':
      return 'team-accepted'
    case 'owner':
      return 'accepted'
    case 'dev':
      return 'dev-accepted'
    default:
      return 'pending'
  }
}

export const labelFor = (kind: BadgeKind, state: Story['state']): string => {
  if (kind === 'blocked') return 'blocked'
  if (kind === 'rework') return 'rework'
  if (kind === 'pending') return 'awaiting verdict'
  if (state === 'accepted') {
    if (kind === 'team-accepted') return 'team-accepted'
    if (kind === 'dev-accepted') return 'dev-accepted'
    return 'owner-accepted'
  }
  if (kind === 'team-accepted') return 'awaiting team'
  if (kind === 'dev-accepted') return 'awaiting dev'
  return 'awaiting owner'
}
