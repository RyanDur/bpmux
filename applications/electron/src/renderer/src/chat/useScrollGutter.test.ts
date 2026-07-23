import { describe, test, expect, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useScrollGutter } from './useScrollGutter'

const stubUlElement = (
  overrides: Partial<{ scrollTop: number; scrollHeight: number; clientHeight: number }> = {}
): HTMLUListElement => {
  const el = {
    scrollTop: 0,
    scrollHeight: 1000,
    clientHeight: 500,
    scrollTo: vi.fn(),
    ...overrides
  }
  return el as unknown as HTMLUListElement
}

describe('useScrollGutter', () => {
  test('initial state: newCount is 0', () => {
    const { result } = renderHook(() =>
      useScrollGutter({ visibleCount: 0, isLoadingOlder: false, hasMoreOlder: false })
    )
    expect(result.current.newCount).toBe(0)
  })

  test('handleScroll fires onAtBottomChange when scroll-position leaves bottom', () => {
    const onAtBottomChange = vi.fn()
    const { result } = renderHook(() =>
      useScrollGutter({
        visibleCount: 0,
        isLoadingOlder: false,
        hasMoreOlder: false,
        onAtBottomChange
      })
    )
    result.current.listRef.current = stubUlElement({ scrollTop: -100 })
    act(() => result.current.handleScroll())
    expect(onAtBottomChange).toHaveBeenCalledWith(false)
  })

  test('handleScroll triggers loadOlder when near top under column-reverse', () => {
    const loadOlder = vi.fn().mockResolvedValue(undefined)
    const { result } = renderHook(() =>
      useScrollGutter({
        visibleCount: 50,
        isLoadingOlder: false,
        hasMoreOlder: true,
        loadOlder
      })
    )
    result.current.listRef.current = stubUlElement({
      scrollTop: -480,
      scrollHeight: 1000,
      clientHeight: 500
    })
    act(() => result.current.handleScroll())
    expect(loadOlder).toHaveBeenCalled()
  })

  test('handleScroll does not trigger loadOlder when already loading', () => {
    const loadOlder = vi.fn().mockResolvedValue(undefined)
    const { result } = renderHook(() =>
      useScrollGutter({
        visibleCount: 50,
        isLoadingOlder: true,
        hasMoreOlder: true,
        loadOlder
      })
    )
    result.current.listRef.current = stubUlElement({ scrollTop: -480 })
    act(() => result.current.handleScroll())
    expect(loadOlder).not.toHaveBeenCalled()
  })

  test('scrollToBottom calls element.scrollTo({top:0})', () => {
    const { result } = renderHook(() =>
      useScrollGutter({ visibleCount: 0, isLoadingOlder: false, hasMoreOlder: false })
    )
    const el = stubUlElement()
    result.current.listRef.current = el
    act(() => result.current.scrollToBottom())
    expect(el.scrollTo).toHaveBeenCalledWith({ top: 0, behavior: 'smooth' })
  })

  test('handleScroll is no-op when listRef.current is null', () => {
    const onAtBottomChange = vi.fn()
    const { result } = renderHook(() =>
      useScrollGutter({
        visibleCount: 0,
        isLoadingOlder: false,
        hasMoreOlder: false,
        onAtBottomChange
      })
    )
    expect(() => act(() => result.current.handleScroll())).not.toThrow()
    expect(onAtBottomChange).not.toHaveBeenCalled()
  })
})
