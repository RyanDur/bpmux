import type { BoardStory, IterationReadiness } from './types'

export const outcome = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 1,
  title: 'An outcome',
  state: 'icebox',
  labels: ['outcome'],
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: '',
  horizon: 'now',
  ...overrides
})

export const buildingIteration = (
  number: number,
  title: string,
  position: number,
  readiness: IterationReadiness,
  outcomeNumber = 1
): BoardStory => ({
  number,
  title,
  state: 'in-progress',
  labels: ['iteration'],
  assignees: [],
  created_at: '2026-06-15T00:00:00Z',
  body: 'goal',
  outcome_membership: { outcome_number: outcomeNumber, position },
  readiness
})

export const checks = (
  drafted: boolean,
  stories_ordered: boolean,
  review_closed: boolean,
  dev_ok: boolean
): IterationReadiness => ({ drafted, stories_ordered, review_closed, dev_ok })
