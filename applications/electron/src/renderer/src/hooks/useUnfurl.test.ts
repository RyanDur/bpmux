import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { extractUrls, useUnfurl } from './useUnfurl'

describe('extractUrls', () => {
  it('extracts and dedups urls, stripping trailing punctuation', () => {
    expect(extractUrls('see https://a.test, and https://a.test).')).toEqual(['https://a.test'])
  })
})

describe('#1499 useUnfurl surfaces resolved data without a mounted-guard ref', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('resolves unfurl data after fetch (bump fires on resolve)', async () => {
    const url = `https://example.test/${Math.random().toString(36).slice(2)}`
    const payload = { url, title: 'Title', description: 'desc' }
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => payload
      } as unknown as Response)
    )
    const { result } = renderHook(() =>
      useUnfurl({ apiBase: 'http://api', authToken: 't', urls: [url] })
    )
    await waitFor(() => expect(result.current.get(url)).toEqual(payload))
  })
})
