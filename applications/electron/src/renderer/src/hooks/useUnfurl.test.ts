import { describe, expect, it } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { extractUrls, useUnfurl } from './useUnfurl'
import { respondJson, startTestHttpServer } from '../../../../test/httpTestServer'

describe('extractUrls', () => {
  it('extracts and dedups urls, stripping trailing punctuation', () => {
    expect(extractUrls('see https://a.test, and https://a.test).')).toEqual(['https://a.test'])
  })
})

describe('#1499 useUnfurl surfaces resolved data without a mounted-guard ref', () => {
  it('resolves unfurl data after fetch (bump fires on resolve)', async () => {
    const url = `https://example.test/${Math.random().toString(36).slice(2)}`
    const payload = { url, title: 'Title', description: 'desc' }
    const server = await startTestHttpServer((_req, res) => respondJson(res, 200, payload))
    try {
      const { result } = renderHook(() =>
        useUnfurl({ apiBase: server.baseUrl, authToken: 't', urls: [url] })
      )
      await waitFor(() => expect(result.current.get(url)).toEqual(payload))
    } finally {
      await server.close()
    }
  })
})
