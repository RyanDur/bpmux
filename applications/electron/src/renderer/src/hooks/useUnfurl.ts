import { useEffect, useRef, useState } from 'react'

// URL extraction from arbitrary message text. Intentionally
// conservative — matches http(s) only, stops at whitespace, drops
// trailing punctuation likely to be part of the surrounding prose.
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

export interface UnfurlData {
  url: string
  title?: string | null
  description?: string | null
  image?: string | null
  site_name?: string | null
  favicon?: string | null
}

interface CacheEntry {
  state: 'loading' | 'resolved'
  data?: UnfurlData
}

// Module-scope cache so the fetch for a URL happens once per app
// lifetime, shared across every message that renders the same URL.
// Keyed by URL. Survives component remounts.
const cache = new Map<string, CacheEntry>()

interface UseUnfurlArgs {
  apiBase: string | undefined
  authToken: string | undefined
  urls: readonly string[]
}

export const useUnfurl = ({
  apiBase,
  authToken,
  urls
}: UseUnfurlArgs): Map<string, UnfurlData | undefined> => {
  // Object state that gets replaced whenever the set of cached
  // results changes, so React rerenders the message row with the
  // new previews.
  const [, bumpVersion] = useState<number>(0)
  const bump = (): void => bumpVersion((v) => v + 1)
  const mountedRef = useRef<boolean>(true)
  useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
    }
  }, [])

  useEffect(() => {
    if (!apiBase) return
    // Async-lazy fetch: for each URL we don't yet have a resolved
    // cache entry for, kick off the fetch + update the cache when
    // it lands. Parallel per URL; no queueing needed at this scale.
    for (const url of urls) {
      const existing = cache.get(url)
      if (existing) continue
      cache.set(url, { state: 'loading' })
      ;(async () => {
        try {
          const res = await fetch(`${apiBase}/api/unfurl?url=${encodeURIComponent(url)}`, {
            headers: { Authorization: `Bearer ${authToken ?? ''}` }
          })
          let data: UnfurlData = { url }
          if (res.ok) {
            try {
              data = (await res.json()) as UnfurlData
            } catch {
              // Invalid JSON — treat as minimal data.
            }
          }
          cache.set(url, { state: 'resolved', data })
          if (mountedRef.current) bump()
        } catch {
          cache.set(url, { state: 'resolved', data: { url } })
          if (mountedRef.current) bump()
        }
      })()
    }
  }, [apiBase, authToken, urls])

  // Build the return map from current cache state.
  const result = new Map<string, UnfurlData | undefined>()
  for (const url of urls) {
    const entry = cache.get(url)
    result.set(url, entry?.state === 'resolved' ? entry.data : undefined)
  }
  return result
}
