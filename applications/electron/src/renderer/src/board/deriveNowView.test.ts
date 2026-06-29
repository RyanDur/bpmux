import { describe, expect, test } from 'vitest'
import { deriveNowView, type NowRow } from './deriveNowView'
import type { BoardIterationData } from './selectIterations'
import type { BoardStory } from './types'

const iteration = (overrides: Partial<BoardIterationData>): BoardIterationData => ({
  number: 9000,
  title: 'Iteration Lyra',
  iteration_order: null,
  is_current: false,
  complete: false,
  readiness: { drafted: false, stories_ordered: false, review_closed: false, dev_ok: false },
  stories: [],
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
    expect(deriveNowView([], []).kind).toBe('empty')
    const undesignated = iteration({ number: 1 })
    expect(deriveNowView([undesignated], []).kind).toBe('empty')
  })

  test('current variant carries the designated iteration and its members', () => {
    const iter = iteration({ number: 200, is_current: true, stories: [9001] })
    const member = story({ number: 9001 })
    const view = deriveNowView([iter], [member])
    expect(view.kind).toBe('current')
    if (view.kind !== 'current') throw new Error('expected current variant')
    expect(view.iteration.number).toBe(200)
    expect(view.members.map((m) => m.story.number)).toEqual([9001])
    expect(view.members[0].membership).toEqual({ iteration_number: 200, position: 1 })
  })

  test('a story missing iteration_membership is excluded, never a silent member', () => {
    const iter = iteration({ number: 200, is_current: true })
    const orphan = story({ number: 9002 })
    const otherIter = story({
      number: 9003,
      iteration_membership: { iteration_number: 999, position: 100 }
    })
    const view = deriveNowView([iter], [orphan, otherIter])
    if (view.kind !== 'current') throw new Error('expected current variant')
    expect(view.members).toEqual([])
  })

  test('an iteration-typed story in the pool is never a member row', () => {
    const iter = iteration({ number: 200, is_current: true, stories: [201] })
    const iterationStoryInPool = story({
      number: 201,
      labels: ['iteration']
    })
    const view = deriveNowView([iter], [iterationStoryInPool])
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
