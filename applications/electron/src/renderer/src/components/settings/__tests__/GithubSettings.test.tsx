import { afterEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { act, cleanup } from '@testing-library/react'
import GithubSettings from '../GithubSettings'
import { dispatchState, resetStateForTest } from '@state/store'
import { mirrorDeltaReceived } from '@state/actions'
import type { SyncStatusWire } from '@transport/types'

afterEach(cleanup)

const healthySync: SyncStatusWire = {
  last_ingest_at: null,
  last_push_at: null,
  pending_events: 0,
  last_error: null,
  dropped_events: [],
  stale: false,
  startup_fail: false
}

const pushSync = (status: SyncStatusWire): void =>
  act(() => {
    dispatchState(mirrorDeltaReceived({ domain: 'sync_poll', status }, '2026-07-28T00:00:00Z'))
  })

const baseProps = {
  apiBase: undefined,
  authToken: undefined,
  ownerNonce: undefined,
  onClose: (): void => {}
}

describe('GithubSettings dialog is declarative (no ref)', () => {
  test('open=true reflects the open attribute and names the surface', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html).toMatch(/<dialog\b[^>]*\bopen\b/)
    expect(html).toContain('aria-label="GitHub settings"')
    expect(html).toContain('aria-label="Close settings"')
  })

  test('open=false omits the open attribute declaratively', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={false} />)
    expect(html).not.toMatch(/<dialog\b[^>]*\bopen\b/)
  })
})

describe('the connect form is complete before any network settles', () => {
  test('owner, repo, and a password-typed token field render with the submit affordance', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html).toContain('data-test="gh-owner"')
    expect(html).toContain('data-test="gh-repo"')
    expect(html).toMatch(
      /data-test="gh-token"[^>]*type="password"|type="password"[^>]*data-test="gh-token"/
    )
    expect(html).toContain('data-test="gh-connect"')
  })

  test('each field reserves its message slot so errors never shift the form', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html.match(/github-settings-field-error/g)).toHaveLength(3)
  })

  test('no static status caption survives on the surface', () => {
    const html = renderToStaticMarkup(<GithubSettings {...baseProps} open={true} />)
    expect(html).not.toContain('always current')
    expect(html).not.toContain('restart')
  })
})

describe('the enterprise reveal', () => {
  test('the checkbox renders and the API base field stays hidden until checked', async () => {
    const { render, screen } = await import('@testing-library/react')
    const { default: userEvent } = await import('@testing-library/user-event')
    render(<GithubSettings {...baseProps} open={true} />)
    expect(screen.getByText('GitHub Enterprise')).toBeTruthy()
    expect(screen.queryByText('API base URL')).toBeNull()
    await userEvent.click(screen.getByRole('checkbox'))
    expect(screen.getByText('API base URL')).toBeTruthy()
  })
})

describe('a refused connection surfaces in the dialog — sync status is the failure surface', () => {
  const statusResponse = (body: Record<string, unknown>): Response =>
    new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'content-type': 'application/json' }
    })

  const openDialog = async (
    status: Record<string, unknown>
  ): Promise<(typeof import('@testing-library/react'))['screen']> => {
    const { render, screen } = await import('@testing-library/react')
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => statusResponse(status))
    )
    render(<GithubSettings {...baseProps} apiBase="http://127.0.0.1:9" open={true} />)
    await screen.findByText(/connected/)
    return screen
  }

  afterEach(() => {
    vi.unstubAllGlobals()
    resetStateForTest()
  })

  test('a failing sync shows the human clause and keeps the raw string in the tooltip', async () => {
    const screen = await openDialog({ state: 'connected', owner: 'acme', repo: 'bpmux' })
    const raw = 'ingest: GitHub refused the token — check it in GitHub settings and reconnect'
    pushSync({ ...healthySync, last_error: raw })
    const line = await screen.findByText(/GitHub refused the token/)
    expect(line.textContent).not.toContain('ingest:')
    expect(line.getAttribute('title')).toBe(raw)
  })

  test('a healthy sync shows no failure line', async () => {
    const screen = await openDialog({ state: 'connected', owner: 'acme', repo: 'bpmux' })
    pushSync(healthySync)
    expect(screen.queryByText(/sync is failing/)).toBeNull()
  })

  test('a disconnected board shows no failure line even if a stale error lingers', async () => {
    const screen = await openDialog({ state: 'disconnected' })
    pushSync({ ...healthySync, last_error: 'ingest: source unreachable' })
    expect(screen.queryByText(/sync is failing/)).toBeNull()
  })
})

describe('validation happens inline, on submit, next to the field', () => {
  const disconnected = (): Response =>
    new Response(JSON.stringify({ state: 'disconnected' }), {
      status: 200,
      headers: { 'content-type': 'application/json' }
    })

  const openEnterpriseForm = async (): Promise<{
    screen: (typeof import('@testing-library/react'))['screen']
    user: ReturnType<(typeof import('@testing-library/user-event'))['default']['setup']>
    fetchSpy: ReturnType<typeof vi.fn>
  }> => {
    const { render, screen } = await import('@testing-library/react')
    const { default: userEvent } = await import('@testing-library/user-event')
    const fetchSpy = vi.fn(async () => disconnected())
    vi.stubGlobal('fetch', fetchSpy)
    const user = userEvent.setup()
    render(<GithubSettings {...baseProps} apiBase="http://127.0.0.1:9" open={true} />)
    await screen.findByText('not connected')
    await user.click(screen.getByRole('checkbox'))
    await user.type(screen.getByLabelText('owner'), 'acme')
    await user.type(screen.getByLabelText('repository'), 'bpmux')
    return { screen, user, fetchSpy }
  }

  const putCalls = (fetchSpy: ReturnType<typeof vi.fn>): unknown[] =>
    fetchSpy.mock.calls.filter(([, init]) => (init as RequestInit | undefined)?.method === 'PUT')

  afterEach(() => vi.unstubAllGlobals())

  test('an empty submit marks every field inline and no request leaves', async () => {
    const { screen, user, fetchSpy } = await openEnterpriseForm()
    await user.clear(screen.getByLabelText('owner'))
    await user.clear(screen.getByLabelText('repository'))
    await user.click(screen.getByText('connect'))
    await screen.findByText('enter the owner')
    expect(screen.getByText('enter the repository')).toBeTruthy()
    expect(screen.getByText('enter a token')).toBeTruthy()
    expect(screen.getByText('enter the API base URL')).toBeTruthy()
    expect(screen.getByLabelText('owner').getAttribute('aria-invalid')).toBe('true')
    expect(putCalls(fetchSpy)).toHaveLength(0)
  })

  test('a missing base URL blocks the submit instead of silently connecting to github.com', async () => {
    const { screen, user, fetchSpy } = await openEnterpriseForm()
    await user.type(screen.getByLabelText('token'), 'ghp_secret')
    await user.click(screen.getByText('connect'))
    await screen.findByText('enter the API base URL')
    expect(screen.queryByText('enter a token')).toBeNull()
    expect(putCalls(fetchSpy)).toHaveLength(0)
  })

  test('correcting a field retires its error without waiting for the next submit', async () => {
    const { screen, user } = await openEnterpriseForm()
    await user.click(screen.getByText('connect'))
    await screen.findByText('enter a token')
    await user.type(screen.getByLabelText('token'), 'ghp_secret')
    expect(screen.queryByText('enter a token')).toBeNull()
    expect(screen.getByText('enter the API base URL')).toBeTruthy()
  })

  test('both present lets the PUT leave with the base_url in the payload', async () => {
    const { screen, user, fetchSpy } = await openEnterpriseForm()
    await user.type(screen.getByLabelText('token'), 'ghp_secret')
    await user.type(screen.getByLabelText('API base URL'), 'https://ghe.example.com/api/v3/')
    await user.click(screen.getByText('connect'))
    const puts = putCalls(fetchSpy)
    expect(puts).toHaveLength(1)
    const [, init] = puts[0] as [unknown, RequestInit]
    expect(JSON.parse(init.body as string)).toMatchObject({
      base_url: 'https://ghe.example.com/api/v3/'
    })
  })
})
