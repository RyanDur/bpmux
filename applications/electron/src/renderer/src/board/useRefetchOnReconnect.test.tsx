import { describe, expect, test } from 'vitest'
import { decideRefetchOnStatus } from './useRefetchOnReconnect'

describe('#511 decideRefetchOnStatus — WS-reconnect refetch policy', () => {
  test('connecting (initial) does NOT set wasDisconnected; no refetch', () => {
    expect(decideRefetchOnStatus('connecting', false)).toEqual({
      wasDisconnected: false,
      shouldRefetch: false
    })
  })

  test('connected without prior-disconnect does NOT refetch (initial connect, not reconnect)', () => {
    expect(decideRefetchOnStatus('connected', false)).toEqual({
      wasDisconnected: false,
      shouldRefetch: false
    })
  })

  test('disconnected sets wasDisconnected; no refetch yet', () => {
    expect(decideRefetchOnStatus('disconnected', false)).toEqual({
      wasDisconnected: true,
      shouldRefetch: false
    })
  })

  test('disconnected→disconnected stays sticky; no refetch', () => {
    expect(decideRefetchOnStatus('disconnected', true)).toEqual({
      wasDisconnected: true,
      shouldRefetch: false
    })
  })

  test('connecting (after a disconnect) preserves wasDisconnected; no refetch yet', () => {
    expect(decideRefetchOnStatus('connecting', true)).toEqual({
      wasDisconnected: true,
      shouldRefetch: false
    })
  })

  test('connected after prior-disconnect FIRES refetch and clears wasDisconnected', () => {
    expect(decideRefetchOnStatus('connected', true)).toEqual({
      wasDisconnected: false,
      shouldRefetch: true
    })
  })
})
