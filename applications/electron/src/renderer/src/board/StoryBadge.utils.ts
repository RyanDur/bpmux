import { type Story } from './types'

export type BadgeKind = 'team-accepted' | 'accepted' | 'dev-accepted' | 'pending' | 'rework'

const DESK_CHECK_FAIL_LABEL = 'desk-check-fail'

export const deriveBadge = (story: Story): BadgeKind | null => {
  if (story.state !== 'acceptance' && story.state !== 'accepted') return null
  const normalized = story.labels.map((l) => l.toLowerCase())
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
