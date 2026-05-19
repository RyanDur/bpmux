import { describe, test, expect } from 'vitest'
import { deriveBadge } from './StoryBadge'
import type { BoardStory } from './types'

const baseStory = (overrides: Partial<BoardStory> = {}): BoardStory => ({
  number: 100,
  title: 'test story',
  state: 'delivered',
  labels: [],
  assignees: [],
  created_at: '2026-05-19T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1437 deriveBadge — acceptor-label-first discrimination', () => {
  test('awaiting team label routes Feature-type to team-accepted', () => {
    const story = baseStory({
      title: '[Feature] new view',
      labels: ['feature', 'awaiting team']
    })
    expect(deriveBadge(story)).toBe('team-accepted')
  })

  test('awaiting owner label routes Bug-type to accepted (owner lane)', () => {
    const story = baseStory({
      title: '[Bug] crash on load',
      labels: ['bug', 'awaiting owner']
    })
    expect(deriveBadge(story)).toBe('accepted')
  })

  test('awaiting team label routes Chore-type to team-accepted', () => {
    const story = baseStory({
      title: '[Chore] refactor module',
      labels: ['chore', 'awaiting team']
    })
    expect(deriveBadge(story)).toBe('team-accepted')
  })

  test('acceptor-label match is case-insensitive', () => {
    const story = baseStory({
      labels: ['feature', 'Awaiting Team']
    })
    expect(deriveBadge(story)).toBe('team-accepted')
  })

  test('falls back to story-type when acceptor-label absent — Feature → accepted', () => {
    const story = baseStory({
      labels: ['feature']
    })
    expect(deriveBadge(story)).toBe('accepted')
  })

  test('falls back to story-type when acceptor-label absent — Bug → team-accepted', () => {
    const story = baseStory({
      labels: ['bug']
    })
    expect(deriveBadge(story)).toBe('team-accepted')
  })

  test('blocked precedence overrides acceptor-label discrimination', () => {
    const story = baseStory({
      labels: ['feature', 'awaiting team', 'blocked']
    })
    expect(deriveBadge(story)).toBe('blocked')
  })

  test('rework precedence overrides acceptor-label discrimination', () => {
    const story = baseStory({
      labels: ['feature', 'awaiting team', 'desk-check-fail']
    })
    expect(deriveBadge(story)).toBe('rework')
  })

  test('returns null for upstream states regardless of labels', () => {
    expect(deriveBadge(baseStory({ state: 'ready', labels: ['awaiting team'] }))).toBeNull()
    expect(deriveBadge(baseStory({ state: 'in-progress', labels: ['awaiting team'] }))).toBeNull()
  })

  test('accepted-state story with awaiting-team label retains team lane', () => {
    const story = baseStory({
      state: 'accepted',
      labels: ['feature', 'awaiting team']
    })
    expect(deriveBadge(story)).toBe('team-accepted')
  })
})
