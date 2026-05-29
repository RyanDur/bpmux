import { describe, expect, test } from 'vitest'
import { deriveNowView, type NowRow } from './deriveNowView'
import type { BoardStory } from './types'

const iterationStory = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9000,
  title: 'Iteration Lyra',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

const story = (overrides: Partial<BoardStory>): BoardStory => ({
  number: 9100,
  title: 'member',
  state: 'ready',
  gh_state: 'open',
  labels: ['feature'],
  assignees: [],
  created_at: '2026-05-18T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1578 deriveNowView — narrowed NOW render-input', () => {
  test('empty variant when no iteration is designated', () => {
    expect(deriveNowView([]).kind).toBe('empty')
    const undesignated = iterationStory({ number: 1 })
    expect(deriveNowView([undesignated]).kind).toBe('empty')
  })

  test('current variant carries the designated iteration and its members', () => {
    const iter = iterationStory({ number: 200, is_current: true })
    const member = story({
      number: 9001,
      iteration_membership: { iteration_number: 200, position: 100 }
    })
    const view = deriveNowView([iter, member])
    expect(view.kind).toBe('current')
    if (view.kind !== 'current') throw new Error('expected current variant')
    expect(view.iteration.number).toBe(200)
    expect(view.members.map((m) => m.story.number)).toEqual([9001])
    expect(view.members[0].membership).toEqual({ iteration_number: 200, position: 100 })
  })

  test('a story missing iteration_membership is excluded, never a silent member', () => {
    const iter = iterationStory({ number: 200, is_current: true })
    const orphan = story({ number: 9002 })
    const otherIter = story({
      number: 9003,
      iteration_membership: { iteration_number: 999, position: 100 }
    })
    const view = deriveNowView([iter, orphan, otherIter])
    if (view.kind !== 'current') throw new Error('expected current variant')
    expect(view.members).toEqual([])
  })

  test('the designated iteration story itself is never a member row', () => {
    const iter = iterationStory({
      number: 200,
      is_current: true,
      iteration_membership: { iteration_number: 200, position: 0 }
    })
    const view = deriveNowView([iter])
    if (view.kind !== 'current') throw new Error('expected current variant')
    expect(view.members).toEqual([])
  })

  test('NowRow.membership is required at the type level — the field cannot be omitted', () => {
    type MembershipIsRequired = undefined extends NowRow['membership']
      ? 'REGRESSION-membership-became-optional'
      : 'required'
    const membershipContract: MembershipIsRequired = 'required'
    expect(membershipContract).toBe('required')
  })
})
