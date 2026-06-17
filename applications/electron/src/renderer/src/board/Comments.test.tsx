import { afterEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { act, cleanup, render, waitFor } from '@testing-library/react'
import Comments from './Comments'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'

const bumpBoard = (): void =>
  dispatchSession(
    mirrorDeltaReceived({ domain: 'board_invalidated', reason: 'board changed' }, '2026-01-01T00:00:00Z')
  )

const getStoryComments = vi.fn()
const postStoryComment = vi.fn()

vi.mock('../api/storiesClient', () => ({
  getStoryComments: (...args: unknown[]) => getStoryComments(...args),
  postStoryComment: (...args: unknown[]) => postStoryComment(...args)
}))

const chainable = (): unknown => {
  const c = {
    onSuccess: () => c,
    onFailure: () => c,
    value: Promise.resolve()
  }
  return c
}

const baseProps = {
  storyNumber: 999,
  apiBase: undefined,
  authToken: undefined,
  usingMock: false,
  onError: () => {}
}

afterEach(() => {
  cleanup()
  resetSessionForTest()
})

describe('#1355 Comments independence', () => {
  test('renders standalone (substrate-axis) — no DrillIn.tsx parent required', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toMatch(/<section[^>]*class="drill-in-comments"/)
    expect(html).toContain('aria-label="Comments"')
  })

  test('renders Comments header heading', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toContain('<h4>Comments')
  })

  test('renders empty state on initial mount when no backend (apiBase=undefined)', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toContain('drill-in-comments-empty')
    expect(html).toContain('no comments yet')
  })

  test('renders the comment submit form regardless of comments state', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toContain('data-test="drill-in-comment-input"')
    expect(html).toContain('data-test="drill-in-comment-send"')
    expect(html).toContain('Cmd+Enter to send')
  })

  test('submit button is disabled when draft is empty (initial state)', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    const sendButtonMatch = html.match(/<button[^>]*data-test="drill-in-comment-send"[^>]*>/)
    expect(sendButtonMatch).not.toBeNull()
    expect(sendButtonMatch?.[0]).toContain('disabled')
  })

  test('renders standalone for different stories with distinct storyNumber prop', () => {
    const a = renderToStaticMarkup(<Comments {...baseProps} storyNumber={101} />)
    const b = renderToStaticMarkup(<Comments {...baseProps} storyNumber={102} />)
    expect(a).toContain('drill-in-comments')
    expect(b).toContain('drill-in-comments')
  })
})

describe('#1456 Comments — refetch survives the ingest-strip (tick replaces marker-scan)', () => {
  const liveProps = {
    storyNumber: 700,
    apiBase: 'http://127.0.0.1:9',
    authToken: 'tok',
    usingMock: false,
    onError: (): void => {}
  }

  test('a board-invalidate tick increment triggers a comments refetch (silent-regression guard)', async () => {
    getStoryComments.mockReset()
    getStoryComments.mockImplementation(() => chainable())
    render(<Comments {...liveProps} />)
    await waitFor(() => expect(getStoryComments).toHaveBeenCalled())
    const before = getStoryComments.mock.calls.length

    act(() => {
      bumpBoard()
    })

    await waitFor(() => expect(getStoryComments.mock.calls.length).toBe(before + 1))
  })
})
