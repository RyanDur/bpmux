import { afterEach, describe, expect, test, vi } from 'vitest'
import { postStoryAccept, postStoryReject } from './storiesClient'

const okResponse = (): Response =>
  new Response(JSON.stringify({ ok: true }), {
    status: 200,
    headers: { 'Content-Type': 'application/json' }
  })

const capturedHeaders = (fetchMock: ReturnType<typeof vi.fn>): Record<string, string> => {
  const init = fetchMock.mock.calls[0]?.[1] as RequestInit | undefined
  return (init?.headers ?? {}) as Record<string, string>
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('board acceptance carries the owner-session nonce', () => {
  test('accept POST attaches X-BPMUX-Owner-Nonce when the launch nonce is present', async () => {
    const fetchMock = vi.fn(async () => okResponse())
    vi.stubGlobal('fetch', fetchMock)

    await postStoryAccept(
      { apiBase: 'http://localhost:1', authToken: 'tok', ownerNonce: 'launch-nonce-xyz' },
      42,
      { would_file_bug: false }
    ).value

    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(capturedHeaders(fetchMock)['X-BPMUX-Owner-Nonce']).toBe('launch-nonce-xyz')
  })

  test('reject POST attaches X-BPMUX-Owner-Nonce when the launch nonce is present', async () => {
    const fetchMock = vi.fn(async () => okResponse())
    vi.stubGlobal('fetch', fetchMock)

    await postStoryReject(
      { apiBase: 'http://localhost:1', authToken: 'tok', ownerNonce: 'launch-nonce-xyz' },
      42,
      { note: 'not yet' }
    ).value

    expect(capturedHeaders(fetchMock)['X-BPMUX-Owner-Nonce']).toBe('launch-nonce-xyz')
  })

  test('no owner-nonce header when the launch nonce is absent', async () => {
    const fetchMock = vi.fn(async () => okResponse())
    vi.stubGlobal('fetch', fetchMock)

    await postStoryAccept({ apiBase: 'http://localhost:1', authToken: 'tok' }, 42, {
      would_file_bug: false
    }).value

    expect(capturedHeaders(fetchMock)['X-BPMUX-Owner-Nonce']).toBeUndefined()
  })
})
