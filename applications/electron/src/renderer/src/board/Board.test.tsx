import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, test } from 'vitest'

import Board from './Board'
import { type Story } from './types'
import { boardIteration, seedIterations } from './Iteration.fixtures'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const pushStories = (stories: Story[]): void =>
  dispatchSession(mirrorDeltaReceived({ domain: 'stories', stories }, '2026-06-16T00:00:00Z'))

const story = (over: Partial<Story>): Story => ({
  id: 1,
  title: 'story',
  state: 'ready',
  labels: [],
  assignees: [],
  created_at: '2026-06-16T00:00:00Z',
  body: '',
  ...over
})

const seedCurrentIteration = (): void =>
  seedIterations([boardIteration({ id: 100, title: 'Crux', is_current: true })])

const seedNoCurrentIteration = (): void => seedIterations([])

const MEMBERS: Story[] = [
  story({
    id: 101,
    title: 'a ready member',
    state: 'ready',
    labels: ['Feature']
  })
]

const NO_ITERATION: Story[] = [
  story({ id: 200, title: 'an iceboxed feature', state: 'icebox', labels: ['Feature'] })
]

const renderBoard = (stories: Story[]): ReturnType<typeof render> => {
  pushStories(stories)
  return render(<Board />)
}

const boardPanel = (): HTMLElement => screen.getByTestId('board-roadmap-panel-board')

beforeEach(() => {
  window.localStorage.clear()
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
    seedCurrentIteration()
    renderBoard(MEMBERS)
    const panel = boardPanel()
    expect(panel.querySelector('[data-test="board-list"]')).toBeTruthy()
    expect(panel.querySelector('[data-test="roadmap"]')).toBeNull()
  })

  test('LIVE on iteration-CLOSE: dropping the current iteration switches the board to the roadmap with no reload (#1720 freshness)', async () => {
    seedCurrentIteration()
    renderBoard(MEMBERS)
    expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeNull()

    act(() => seedNoCurrentIteration())

    await waitFor(() => expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeTruthy())
  })

  test('LIVE on iteration-OPEN: designating a current iteration switches the roadmap fallback back to the board with no reload', async () => {
    seedNoCurrentIteration()
    renderBoard(NO_ITERATION)
    expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeTruthy()

    act(() => seedCurrentIteration())

    await waitFor(() => expect(boardPanel().querySelector('[data-test="roadmap"]')).toBeNull())
  })
})
