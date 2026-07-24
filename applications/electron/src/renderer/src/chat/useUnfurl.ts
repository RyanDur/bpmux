import { useEffect, useState } from 'react'
import { http } from '../transport/http'

const URL_RE = /https?:\/\/[^\s<>"')(]+/g
const TRAILING_PUNCT_RE = /[.,!?;:)\]'"*]+$/

export const extractUrls = (text: string): string[] => {
  const matches = text.match(URL_RE) ?? []
  const seen = new Set<string>()
  const out: string[] = []
  for (const raw of matches) {
    const cleaned = raw.replace(TRAILING_PUNCT_RE, '')
    if (cleaned.length === 0 || seen.has(cleaned)) continue
    seen.add(cleaned)
    out.push(cleaned)
  }
  return out
}

export type UnfurlData = {
  url: string
  title?: string | null
  description?: string | null
  image?: string | null
  site_name?: string | null
  favicon?: string | null
}

type CacheEntry = {
  state: 'loading' | 'resolved'
  data?: UnfurlData
}

const cache = new Map<string, CacheEntry>()

type UseUnfurlArgs = {
  apiBase: string | undefined
  authToken: string | undefined
  urls: readonly string[]
}

export const useUnfurl = ({
  apiBase,
  authToken,
  urls
}: UseUnfurlArgs): Map<string, UnfurlData | undefined> => {
  const [, bumpVersion] = useState<number>(0)
  const bump = (): void => bumpVersion((v) => v + 1)

  useEffect(() => {
    if (!apiBase) return
    for (const url of urls) {
      const existing = cache.get(url)
      if (existing) continue
      cache.set(url, { state: 'loading' })
      http
        .get<UnfurlData>({ apiBase, authToken }, `/api/unfurl?url=${encodeURIComponent(url)}`)
        .onSuccess((data) => {
          cache.set(url, { state: 'resolved', data })
          bump()
        })
        .onFailure(() => {
          cache.set(url, { state: 'resolved', data: { url } })
          bump()
        })
    }
  }, [apiBase, authToken, urls])

  const result = new Map<string, UnfurlData | undefined>()
  for (const url of urls) {
    const entry = cache.get(url)
    result.set(url, entry?.state === 'resolved' ? entry.data : undefined)
  }
  return result
}
