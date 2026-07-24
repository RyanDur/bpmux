import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useSelector } from './useSelector'
import { dispatchState, resetStateForTest } from './store'
import { mirrorDeltaReceived } from './actions'
import { selectRoadmap, type RoadmapCard } from '../board/roadmap/selectRoadmap'
const ts = '2026-06-27T00:00:00Z'
const nowCard: RoadmapCard = {
  id: 1,
  name: 'Now',
  why: {},
  horizon: 'now',
  state: 'planned',
  gap_count: 0,
  tone: 'now'
}

const emitRoadmap = (roadmap: RoadmapCard[]): void =>
  act(() => dispatchState(mirrorDeltaReceived({ domain: 'roadmap', roadmap }, ts)))

afterEach(() => {
  act(() => resetStateForTest())
})

describe('useSelector — selector over the local store', () => {
  it('returns whatever the selector returns from current state', () => {
    const { result } = renderHook(() => useSelector((s) => s.outcomes.length))
    expect(result.current).toBe(0)
  })

  it('re-emits to the component when the backend emits a roadmap delta', () => {
    const { result } = renderHook(() => useSelector(selectRoadmap))
    expect(result.current.bands).toEqual([])
    emitRoadmap([nowCard])
    expect(result.current.bands.map((b) => b.horizon)).toEqual(['now'])
    expect(result.current.bands[0].outcomes[0].name).toBe('Now')
  })

  it('returns a referentially stable result across re-renders while state is unchanged', () => {
    emitRoadmap([nowCard])
    const { result, rerender } = renderHook(() => useSelector(selectRoadmap))
    const first = result.current
    rerender()
    expect(result.current).toBe(first)
  })
})
