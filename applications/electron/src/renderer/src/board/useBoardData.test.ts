import { describe, expect, test } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardData } from './useBoardData'
import type { BoardResponse, BoardStory } from './types'

const ITER = 9000

const iterationStory = (): BoardStory => ({
  number: ITER,
  title: 'Iteration Current',
  state: 'icebox',
  gh_state: 'open',
  labels: ['iteration'],
  is_current: true,
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

const ready = (number: number, position: number): BoardStory => ({
  number,
  title: `s${number}`,
  state: 'ready',
  labels: ['Feature', 'ready'],
  iteration_membership: { iteration_number: ITER, position },
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

describe('#1524 useBoardData — ready group ordered by the shared canonical (survives reload)', () => {
  test('orders the ready group by parent-iteration position, not array order', () => {
    const data: BoardResponse = {
      fetched_at: '2026-05-10T00:00:00Z',
      stories: [iterationStory(), ready(701, 300), ready(702, 100), ready(703, 200)]
    }
    const { result } = renderHook(() => useBoardData(data))
    const readyNums = (result.current.grouped.get('ready') ?? []).map((s) => s.number)
    expect(readyNums).toEqual([702, 703, 701])
  })
})
