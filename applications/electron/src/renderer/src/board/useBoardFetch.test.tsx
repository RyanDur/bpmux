import { afterEach, describe, expect, test } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'

import { useBoardFetch } from './useBoardFetch'
import type { BoardResponse } from './types'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const pushSnapshot = (snapshot: BoardResponse): void =>
  dispatchSession(
    mirrorDeltaReceived(
      { domain: 'board_snapshot', stories: snapshot.stories },
      snapshot.fetched_at
    )
  )

afterEach(() => {
  resetSessionForTest()
})

describe('#1720 useBoardFetch — a pushed board-snapshot becomes the data (render=f(substrate))', () => {
  test('a board-snapshot push populates board data over WS (no REST)', async () => {
    const { result } = renderHook(() => useBoardFetch({ apiBase: 'http://127.0.0.1:9' }))

    const pushed: BoardResponse = {
      stories: [
        {
          number: 7,
          title: 'pushed-live',
          state: 'delivered',
          labels: [],
          assignees: [],
          created_at: '2026-06-13T00:00:00Z',
          body: ''
        }
      ],
      fetched_at: '2026-06-13T00:00:00Z'
    }
    act(() => pushSnapshot(pushed))

    await waitFor(() =>
      expect(result.current.data?.stories?.[0]).toMatchObject({ number: 7, state: 'delivered' })
    )
    expect(result.current.loading).toBe(false)
  })
})
