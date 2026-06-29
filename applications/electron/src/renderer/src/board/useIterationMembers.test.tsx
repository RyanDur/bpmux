import { describe, expect, test } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useIterationMembers } from './useIterationMembers'
import { boardIteration } from './Iteration.fixtures'
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

describe('useIterationMembers attributes by the iterations ordered stories (parent-holds-order)', () => {
  test('a story absent from the iterations stories list is not a member', () => {
    const outsider = makeStory({ number: 2001 })
    const { result } = renderHook(() =>
      useIterationMembers([outsider], [boardIteration({ number: 2000, stories: [] })])
    )
    expect(result.current.getMembersFor(2000)).toEqual([])
  })

  test('attributes members from the iterations stories, in that order', () => {
    const first = makeStory({ number: 2001 })
    const second = makeStory({ number: 2002 })
    const { result } = renderHook(() =>
      useIterationMembers(
        [second, first],
        [boardIteration({ number: 2000, stories: [2001, 2002] })]
      )
    )
    const members = result.current.getMembersFor(2000)
    expect(members.map((m) => m.story.number)).toEqual([2001, 2002])
  })
})
