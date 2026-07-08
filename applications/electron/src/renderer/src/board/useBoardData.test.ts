import { describe, expect, test } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useBoardData } from './useBoardData'
import { type Story } from './types'
const ITER = 9000

const iterationStory = (): Story => ({
  id: ITER,
  title: 'Iteration Current',
  state: 'icebox',
  labels: ['iteration'],
  is_current: true,
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

const ready = (id: number): Story => ({
  id: id,
  title: `s${id}`,
  state: 'ready',
  labels: ['Feature', 'ready'],
  assignees: [],
  created_at: '2026-05-10T00:00:00Z',
  body: ''
})

describe('useBoardData — ready group ordered by the parent-held iteration.stories order', () => {
  test('orders the ready group by index in the current iteration stories, not array order', () => {
    const stories: Story[] = [iterationStory(), ready(701), ready(702), ready(703)]
    const { result } = renderHook(() => useBoardData(stories, [], [702, 703, 701]))
    const readyNums = (result.current.grouped.get('ready') ?? []).map((s) => s.id)
    expect(readyNums).toEqual([702, 703, 701])
  })

  test('ready stories absent from the order trail the members, tie-broken by id', () => {
    const stories: Story[] = [iterationStory(), ready(701), ready(702), ready(703)]
    const { result } = renderHook(() => useBoardData(stories, [], [703]))
    const readyNums = (result.current.grouped.get('ready') ?? []).map((s) => s.id)
    expect(readyNums).toEqual([703, 701, 702])
  })
})
