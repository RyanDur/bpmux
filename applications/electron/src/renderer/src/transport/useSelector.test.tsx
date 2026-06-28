import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useSelector } from './useSelector'
import { dispatchSession, resetSessionForTest } from './session/store'
import { mirrorDeltaReceived } from './session/actions'
import { selectRoadmap, type RoadmapOutcomeData } from '../board/selectRoadmap'

const ts = '2026-06-27T00:00:00Z'
const nowOutcome: RoadmapOutcomeData = {
  number: 1,
  title: 'Now',
  body: '',
  horizon: 'now',
  horizon_order: null,
  outcome_readiness: null,
  iterations: []
}

const emitOutcomes = (outcomes: RoadmapOutcomeData[]): void =>
  act(() => dispatchSession(mirrorDeltaReceived({ domain: 'outcomes', outcomes }, ts)))

afterEach(() => {
  act(() => resetSessionForTest())
})

describe('useSelector — selector over the local store', () => {
  it('returns whatever the selector returns from current state', () => {
    const { result } = renderHook(() => useSelector((s) => s.outcomes.length))
    expect(result.current).toBe(0)
  })

  it('re-emits to the component when the backend emits an outcomes delta', () => {
    const { result } = renderHook(() => useSelector(selectRoadmap))
    expect(result.current.bands).toEqual([])
    emitOutcomes([nowOutcome])
    expect(result.current.bands.map((b) => b.horizon)).toEqual(['now'])
    expect(result.current.bands[0].outcomes[0].name).toBe('Now')
  })

  it('returns a referentially stable result across re-renders while state is unchanged', () => {
    emitOutcomes([nowOutcome])
    const { result, rerender } = renderHook(() => useSelector(selectRoadmap))
    const first = result.current
    rerender()
    expect(result.current).toBe(first)
  })
})
