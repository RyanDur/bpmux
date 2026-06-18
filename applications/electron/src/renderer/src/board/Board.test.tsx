import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'

const getBoard = vi.fn()
vi.mock('../api/boardClient', () => ({
  getBoard: (...args: unknown[]) => getBoard(...args)
}))

import Board from './Board'
import type { BoardResponse, BoardStory } from './types'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const pushSnapshot = (snapshot: BoardResponse): void =>
  dispatchSession(
    mirrorDeltaReceived(
      { domain: 'board_snapshot', stories: snapshot.stories },
      snapshot.fetched_at
    )
  )

const inertChain = (): unknown => {
  const chain = { onSuccess: () => chain, onFailure: () => chain }
  return chain
}

const story = (over: Partial<BoardStory>): BoardStory => ({
  number: 1,
  title: 'story',
  state: 'ready',
  labels: [],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  ...over
})

const ITERATION_ACTIVE: BoardResponse = {
  fetched_at: '2026-06-16T00:00:00Z',
  stories: [
    story({
      number: 100,
      title: 'Crux',
      state: 'in-progress',
      labels: ['iteration'],
      gh_state: 'open',
      is_current: true
    }),
    story({ number: 101, title: 'a ready member', state: 'ready', labels: ['Feature'] })
  ]
}

const NO_ITERATION: BoardResponse = {
  fetched_at: '2026-06-16T00:00:01Z',
  stories: [
    story({ number: 200, title: 'an iceboxed feature', state: 'icebox', labels: ['Feature'] })
  ]
}

const renderBoard = (snapshot: BoardResponse): ReturnType<typeof render> => {
  pushSnapshot(snapshot)
  return render(<Board />)
}

const boardPanel = (): HTMLElement => screen.getByTestId('board-roadmap-panel-board')

beforeEach(() => {
  window.localStorage.clear()
  getBoard.mockReset()
  getBoard.mockImplementation(() => inertChain())
  window.bpmux = { wsUrl: 'ws://test.invalid', authToken: 'tok', apiBase: 'http://test.invalid' }
})

afterEach(() => {
  cleanup()
  resetSessionForTest()
  delete window.bpmux
})

describe('#1684 no active iteration → the board panel defaults to the #1704 roadmap-view', () => {
  test('with no active iteration the board panel renders the roadmap-view, not a dead/empty board', () => {
    renderBoard(NO_ITERATION)
    const panel = boardPanel()
    expect(panel.querySelector('[data-test="roadmap"]')).toBeTruthy()
    expect(panel.querySelector('[data-test="board-now-empty"]')).toBeNull()
    expect(panel.querySelector('[data-test="board-list"]')).toBeTruthy()
  })

  test('the Board tab stays the selected one (mockup State 3: "Board" centered, board fallback under it)', () => {
    renderBoard(NO_ITERATION)
    const boardRadio = screen
      .getByTestId('board-roadmap-tab-board')
      .querySelector('input[type="radio"]') as HTMLInputElement
    expect(boardRadio.checked).toBe(true)
  })

  test('an active iteration shows the board (the in-iteration state), NOT the roadmap fallback', () => {
    renderBoard(ITERATION_ACTIVE)
    const panel = boardPanel()
    expect(panel.querySelector('[data-test="board-list"]')).toBeTruthy()
    expect(panel.querySelector('[data-test="roadmap"]')).toBeNull()
  })

  test('LIVE on iteration-CLOSE: a pushed snapshot with no current iteration switches the board to the roadmap with no reload (#1720 freshness)', async () => {
    renderBoard(ITERATION_ACTIVE)
    expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeNull()

    act(() => pushSnapshot(NO_ITERATION))

    await waitFor(() => expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeTruthy())
  })

  test('LIVE on iteration-OPEN: a pushed snapshot with a current iteration switches the roadmap fallback back to the board with no reload', async () => {
    renderBoard(NO_ITERATION)
    expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeTruthy()

    act(() => pushSnapshot(ITERATION_ACTIVE))

    await waitFor(() => expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeNull())
  })
})
