import { afterEach, describe, expect, test } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'

import { useBoardFetch } from './useBoardFetch'
import { type Story } from './types'
import { dispatchState, resetStateForTest } from '../transport/state/store'
import { mirrorDeltaReceived } from '../transport/state/actions'

const pushStories = (stories: Story[]): void =>
  dispatchState(mirrorDeltaReceived({ domain: 'stories', stories }, '2026-06-13T00:00:00Z'))

afterEach(() => {
  resetStateForTest()
})

describe('#1720 useBoardFetch — a pushed stories delta becomes the data (render=f(substrate))', () => {
  test('a stories delta populates board data over WS (no REST)', async () => {
    const { result } = renderHook(() => useBoardFetch({ apiBase: 'http://127.0.0.1:9' }))

    const pushed: Story[] = [
      {
        id: 7,
        title: 'pushed-live',
        state: 'acceptance',
        labels: [],
        assignees: [],
        created_at: '2026-06-13T00:00:00Z',
        body: ''
      }
    ]
    act(() => pushStories(pushed))

    await waitFor(() =>
      expect(result.current.data?.[0]).toMatchObject({ id: 7, state: 'acceptance' })
    )
    expect(result.current.loading).toBe(false)
  })
})
