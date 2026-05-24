import { describe, expect, test } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useIterationMembers } from './useIterationMembers'
import type { BoardStory } from './types'

const makeStory = (overrides: Partial<BoardStory> & { number: number }): BoardStory => ({
  title: `story-${overrides.number}`,
  state: 'icebox',
  labels: [],
  assignees: [],
  created_at: '2026-05-20T00:00:00Z',
  body: '',
  ...overrides
})

describe('#1553 useIterationMembers attributes by iteration identity, not name', () => {
  test('a new iteration does not inherit a retired same-named iteration members', () => {
    const legacyOnly = makeStory({ number: 2001, labels: ['iteration-Carina.100'] })
    const { result } = renderHook(() => useIterationMembers([legacyOnly]))
    expect(result.current.getMembersFor(2000)).toEqual([])
  })

  test('attributes a member by canonical identity across a rename', () => {
    const bound = makeStory({
      number: 2001,
      iteration_membership: { iteration_number: 2000, position: 1 }
    })
    const { result } = renderHook(() => useIterationMembers([bound]))
    const members = result.current.getMembersFor(2000)
    expect(members.map((m) => m.story.number)).toEqual([2001])
  })
})
