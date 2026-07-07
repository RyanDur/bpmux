import { type Epic, type Story, type StoryState } from './types'
export type EpicSort = 'progress' | 'created'

export interface StateBreakdown {
  total: number
  accepted: number
  retired: number
  acceptance: number
  inProgress: number
  deskCheck: number
  ready: number
  icebox: number
}

export const breakdownFor = (epic: Epic, all: Story[]): StateBreakdown => {
  const children = all.filter((s) => s.parent_epic === epic.id)
  const count = (state: StoryState): number => children.filter((s) => s.state === state).length
  return {
    total: children.length,
    accepted: count('accepted'),
    retired: count('retired'),
    acceptance: count('acceptance') + count('ready-for-accept'),
    inProgress: count('in-progress') + count('needs-fixes'),
    deskCheck: count('desk-check'),
    ready: count('ready') + count('rejected'),
    icebox: count('icebox')
  }
}

export const progressDenominatorFor = (b: StateBreakdown): number =>
  b.accepted + b.acceptance + b.inProgress + b.deskCheck + b.ready + b.icebox

export const extractDescription = (body: string): string => {
  if (!body.trim()) return ''
  const userJobMatch = body.match(/##\s+User job\s*\n+([\s\S]+?)(?:\n##\s|$)/i)
  if (userJobMatch && userJobMatch[1]) {
    const para = userJobMatch[1].trim().split(/\n{2,}/)[0]
    if (para) return para.trim()
  }
  const firstPara = body.trim().split(/\n{2,}/)[0]
  return firstPara ? firstPara.trim() : ''
}

const EPIC_TITLE_PREFIX_RE = /^\s*\[\s*epic\s*\]\s*/i

export const stripEpicPrefix = (title: string): string => title.replace(EPIC_TITLE_PREFIX_RE, '')

export const breakdownAriaLabel = (b: StateBreakdown): string => {
  const denom = progressDenominatorFor(b)
  return `Progress: ${b.accepted} of ${denom} children needing completion accepted; ${b.acceptance} in acceptance, ${b.inProgress} in progress, ${b.deskCheck} in desk-check, ${b.ready} ready, ${b.icebox} in icebox, ${b.retired} retired (scope-removed; excluded from progress denominator)`
}

export const isOpenByDefault = (derived: StoryState): boolean =>
  derived !== 'accepted' && derived !== 'icebox'

const progressRatioFor = (epic: Epic, all: Story[]): number => {
  const breakdown = breakdownFor(epic, all)
  const denom = progressDenominatorFor(breakdown)
  return denom === 0 ? 0 : breakdown.accepted / denom
}

export const sortEpics = (epics: Epic[], sort: EpicSort, all: Story[]): Epic[] =>
  [...epics].sort((a, b) => {
    if (sort === 'created') return a.created_at.localeCompare(b.created_at)
    return progressRatioFor(b, all) - progressRatioFor(a, all)
  })
