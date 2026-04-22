import { format } from 'date-fns'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { ChatMessage } from '../transport/types'
import './Search.css'

interface Props {
  apiBase: string | undefined
  authToken: string | undefined
  onClose: () => void
}

interface SearchResult {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function Search({ apiBase, authToken, onClose }: Props): React.JSX.Element {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResult[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    inputRef.current?.focus()
    inputRef.current?.select()
  }, [])

  useEffect(() => {
    return () => abortRef.current?.abort()
  }, [])

  const runSearch = useCallback(
    async (q: string): Promise<void> => {
      if (!apiBase) {
        setError('backend not reachable')
        setResults([])
        return
      }
      const trimmed = q.trim()
      if (!trimmed) {
        setResults(null)
        setError(null)
        return
      }
      abortRef.current?.abort()
      const controller = new AbortController()
      abortRef.current = controller
      setBusy(true)
      setError(null)
      try {
        const url = `${apiBase}/api/search?query=${encodeURIComponent(trimmed)}`
        const res = await fetch(url, {
          headers: { Authorization: `Bearer ${authToken ?? ''}` },
          signal: controller.signal
        })
        if (!res.ok) {
          setError(`HTTP ${res.status}`)
          setResults([])
          return
        }
        const json = (await res.json()) as ChatMessage[] | SearchResult[]
        setResults(json as SearchResult[])
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
        setError(err instanceof Error ? err.message : String(err))
        setResults([])
      } finally {
        setBusy(false)
      }
    },
    [apiBase, authToken]
  )

  useEffect(() => {
    const id = setTimeout(() => {
      void runSearch(query)
    }, 180)
    return () => clearTimeout(id)
  }, [query, runSearch])

  const handleKeyDown = (event: React.KeyboardEvent<HTMLElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault()
      onClose()
    }
  }

  return (
    <section
      className="search"
      data-test="search-overlay"
      aria-label="Search messages"
      onKeyDown={handleKeyDown}
    >
      <header className="search-header">
        <input
          ref={inputRef}
          type="search"
          className="search-input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search messages…"
          aria-label="Search query"
          autoComplete="off"
          spellCheck={false}
          data-test="search-input"
        />
        <button
          type="button"
          className="search-close"
          onClick={onClose}
          aria-label="Close search"
        >
          ✕
        </button>
      </header>

      {error && (
        <p className="search-error" role="alert">
          Search failed: {error}
        </p>
      )}

      {busy && results === null && <p className="search-hint">searching…</p>}

      {results !== null && results.length === 0 && !busy && (
        <p className="search-hint">no results</p>
      )}

      {results !== null && results.length > 0 && (
        <ul className="search-results" data-test="search-results">
          {results.map((r, i) => (
            <li key={`${r.ts}-${i}`} className="search-result">
              <header className="search-result-meta">
                <cite className="search-result-from">{shortName(r.from)}</cite>
                <time dateTime={r.ts} className="search-result-ts">
                  {format(new Date(r.ts), 'MMM d · h:mm a zzz')}
                </time>
              </header>
              <p className="search-result-text">{r.text}</p>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

export default Search
