import { afterEach, describe, expect, test } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'

import { useBoardFetch } from './useBoardFetch'
import { asStoryId, type Story } from './types'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const pushStories = (stories: Story[]): void =>
  dispatchSession(mirrorDeltaReceived({ domain: 'stories', stories }, '2026-06-13T00:00:00Z'))

afterEach(() => {
  resetSessionForTest()
})

describe('#1720 useBoardFetch — a pushed stories delta becomes the data (render=f(substrate))', () => {
  test('a stories delta populates board data over WS (no REST)', async () => {
    const { result } = renderHook(() => useBoardFetch({ apiBase: 'http://127.0.0.1:9' }))

    const pushed: Story[] = [
      {
        id: asStoryId(7),
        title: 'pushed-live',
        state: 'delivered',
        labels: [],
        assignees: [],
        created_at: '2026-06-13T00:00:00Z',
        body: ''
      }
    ]
    act(() => pushStories(pushed))

    await waitFor(() =>
      expect(result.current.data?.[0]).toMatchObject({ id: 7, state: 'delivered' })
    )
    expect(result.current.loading).toBe(false)
  })
})
