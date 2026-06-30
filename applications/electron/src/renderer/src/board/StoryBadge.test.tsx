import { describe, test, expect } from 'vitest'
import { deriveBadge } from './StoryBadge.utils'
import type { Story } from './types'
const baseStory = (overrides: Partial<Story> = {}): Story => ({
  id: 100,
  title: 'test story',
  state: 'delivered',
  labels: [],
  assignees: [],
  created_at: '2026-05-19T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1551 deriveBadge — acceptor-class from typed field, honest-neutral when absent', () => {
  test('acceptor=team routes to team-accepted', () => {
    expect(deriveBadge(baseStory({ acceptor: 'team' }))).toBe('team-accepted')
  })

  test('acceptor=owner routes to accepted (owner lane)', () => {
    expect(deriveBadge(baseStory({ acceptor: 'owner' }))).toBe('accepted')
  })

  test('acceptor=dev routes to dev-accepted (NOT collapsed to team)', () => {
    expect(deriveBadge(baseStory({ acceptor: 'dev' }))).toBe('dev-accepted')
  })

  test('delivered with no acceptor → pending (honest-neutral, not a guess)', () => {
    expect(deriveBadge(baseStory({ state: 'delivered', acceptor: undefined }))).toBe('pending')
  })

  test('badge is NOT inferred from story-type — a bug with no acceptor is pending, not team-accepted', () => {
    expect(deriveBadge(baseStory({ state: 'delivered', labels: ['bug'] }))).toBe('pending')
    expect(deriveBadge(baseStory({ state: 'delivered', labels: ['feature'] }))).toBe('pending')
  })

  test('blocked precedence overrides acceptor', () => {
    expect(
      deriveBadge(baseStory({ state: 'delivered', acceptor: 'team', labels: ['blocked'] }))
    ).toBe('blocked')
  })

  test('rework precedence overrides acceptor', () => {
    expect(
      deriveBadge(baseStory({ state: 'delivered', acceptor: 'team', labels: ['desk-check-fail'] }))
    ).toBe('rework')
  })

  test('no badge for non-delivered/accepted states even with an acceptor', () => {
    expect(deriveBadge(baseStory({ state: 'ready', acceptor: 'team' }))).toBeNull()
    expect(deriveBadge(baseStory({ state: 'in-progress', acceptor: 'team' }))).toBeNull()
  })

  test('accepted-state retains its acceptor lane', () => {
    expect(deriveBadge(baseStory({ state: 'accepted', acceptor: 'team' }))).toBe('team-accepted')
    expect(deriveBadge(baseStory({ state: 'accepted', acceptor: 'dev' }))).toBe('dev-accepted')
    expect(deriveBadge(baseStory({ state: 'accepted', acceptor: 'owner' }))).toBe('accepted')
  })
})
