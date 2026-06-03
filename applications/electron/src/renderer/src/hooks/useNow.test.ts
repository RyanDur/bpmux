import { afterEach, describe, expect, test, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useNow } from './useNow'

afterEach(() => {
  vi.useRealTimers()
})

describe('useNow — a live clock for recency cues', () => {
  test('returns the current time at mount', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-06-03T12:00:00.000Z'))
    const { result } = renderHook(() => useNow(30_000))
    expect(result.current).toBe(Date.parse('2026-06-03T12:00:00.000Z'))
  })

  test('advances on each interval tick so recency stays fresh without new messages', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-06-03T12:00:00.000Z'))
    const { result } = renderHook(() => useNow(30_000))
    const initial = result.current
    act(() => {
      vi.advanceTimersByTime(30_000)
    })
    expect(result.current).toBe(initial + 30_000)
  })
})
