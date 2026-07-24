import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import Comments from '../Comments'
import {
  BrowserWebSocket,
  startTestWsServer,
  type TestWsConnection,
  type TestWsServer
} from '../../../../../../../test/wsTestServer'
import { startConnection, getStateSnapshot, resetStateForTest } from '@state'
import { dispatchState } from '@state/store'
import { mirrorDeltaReceived } from '@state/actions'
const baseProps = {
  storyId: 999,
  apiBase: undefined,
  authToken: undefined,
  onError: () => {}
}

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('Comments independence', () => {
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
})

describe('Comments reads over the live session (real socket + store)', () => {
  const liveProps = {
    storyId: 700,
    apiBase: 'http://backend.invalid',
    authToken: 'tok',
    onError: (): void => {}
  }

  const comment = (over: Record<string, unknown> = {}): Record<string, unknown> => ({
    id: 7700,
    author: 'designer',
    body: 'the selection-based proposal',
    created_at: '2026-04-22T15:20:00Z',
    ...over
  })

  let server: TestWsServer
  let conn: TestWsConnection
  let answered = 0

  const serveComments = async (comments: Record<string, unknown>[]): Promise<void> => {
    await waitFor(() => expect(conn.received.length).toBeGreaterThan(answered))
    answered = conn.received.length
    act(() => {
      conn.send(JSON.stringify({ type: 'comments_hydrated', story_id: 700, comments }))
    })
  }

  beforeEach(async () => {
    vi.stubGlobal('WebSocket', BrowserWebSocket)
    server = await startTestWsServer()
    resetStateForTest()
    startConnection({ url: server.url })
    conn = await server.waitForConnection()
    await waitFor(() => expect(getStateSnapshot().status).toBe('connected'))
    await waitFor(() => expect(conn.received.length).toBeGreaterThanOrEqual(1))
    answered = conn.received.length
  })

  afterEach(async () => {
    resetStateForTest()
    await server.close()
    vi.unstubAllGlobals()
  })

  test('displays the comments the backend serves for this story', async () => {
    render(<Comments {...liveProps} />)
    await serveComments([comment()])
    expect(await screen.findByText('the selection-based proposal')).toBeTruthy()
    expect(screen.getByText('designer')).toBeTruthy()
  })

  test('re-reads the latest comments when the board changes (cross-client freshness)', async () => {
    render(<Comments {...liveProps} />)
    await serveComments([comment({ id: 1, body: 'first proposal' })])
    expect(await screen.findByText('first proposal')).toBeTruthy()

    act(() => {
      dispatchState(
        mirrorDeltaReceived(
          { domain: 'board_invalidated', reason: 'board changed' },
          '2026-01-01T00:00:00Z'
        )
      )
    })

    await serveComments([
      comment({ id: 1, body: 'first proposal' }),
      comment({ id: 2, body: 'second proposal' })
    ])
    expect(await screen.findByText('second proposal')).toBeTruthy()
    expect(screen.getByText('first proposal')).toBeTruthy()
  })
})
