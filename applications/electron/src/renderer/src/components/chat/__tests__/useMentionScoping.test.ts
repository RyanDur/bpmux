import { describe, test, expect, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useMentionScoping } from '../useMentionScoping'

describe('useMentionScoping', () => {
  test('mentionRoles wraps presenceRoles in a Set', () => {
    const { result } = renderHook(() => useMentionScoping(['dev', 'pm', 'owner']))
    expect(result.current.mentionRoles).toBeInstanceOf(Set)
    expect(result.current.mentionRoles.has('dev')).toBe(true)
    expect(result.current.mentionRoles.has('pm')).toBe(true)
    expect(result.current.mentionRoles.has('designer')).toBe(false)
  })

  test('handleMention dispatches recipient via onSelectRecipient', () => {
    const onSelect = vi.fn()
    const { result } = renderHook(() => useMentionScoping(['dev'], onSelect))
    act(() => result.current.handleMention('pm'))
    expect(onSelect).toHaveBeenCalledWith(['pm'])
  })

  test('handleMention is no-op when onSelectRecipient absent', () => {
    const { result } = renderHook(() => useMentionScoping(['dev']))
    expect(() => act(() => result.current.handleMention('pm'))).not.toThrow()
  })

  test('empty presenceRoles yields empty Set', () => {
    const { result } = renderHook(() => useMentionScoping([]))
    expect(result.current.mentionRoles.size).toBe(0)
  })
})
