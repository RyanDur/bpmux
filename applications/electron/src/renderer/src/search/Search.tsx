import { format } from 'date-fns'
import { useCallback, useEffect, useRef, useState } from 'react'
import './Search.css'

interface Props {
  apiBase: string | undefined
  authToken: string | undefined
  onClose: () => void
  onOpenStory: (storyNumber: number, commentId?: number) => void
}

interface MessageResult {
  from: string
  text: string
  ts: string
  to?: string[] | null
  addressee?: string | null
  message_type?: string | null
}

interface StoryResult {
  number: number
  title: string
  snippet: string
  matched_in: 'title' | 'body'
}

interface CommentResult {
  story_number: number
  comment_id: number
  author: string
  snippet: string
}

// Full search response — #344 collapses all three sources into one
// RTT so the UI doesn't progressive-reveal.
interface SearchResponse {
  messages: MessageResult[]
  stories: StoryResult[]
  comments: CommentResult[]
}

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function Search({ apiBase, authToken, onClose, onOpenStory }: Props): React.JSX.Element {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResponse | null>(null)
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
        setResults(null)
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
          setResults({ messages: [], stories: [], comments: [] })
          return
        }
        const json = (await res.json()) as SearchResponse
        // Defensive: older backends or partial results may omit
        // sections; normalise so the render code doesn't null-check
        // at every section.
        setResults({
          messages: json.messages ?? [],
          stories: json.stories ?? [],
          comments: json.comments ?? []
        })
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
        setError(err instanceof Error ? err.message : String(err))
        setResults({ messages: [], stories: [], comments: [] })
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

  const handleStoryClick = (number: number): void => {
    onOpenStory(number)
    onClose()
  }
  const handleCommentClick = (storyNumber: number, commentId: number): void => {
    onOpenStory(storyNumber, commentId)
    onClose()
  }

  const totalHits = results
    ? results.messages.length + results.stories.length + results.comments.length
    : 0

  return (
    <section
      className="search"
      data-test="search-overlay"
      aria-label="Search messages, stories, and comments"
      onKeyDown={handleKeyDown}
    >
      <header className="search-header">
        <input
          ref={inputRef}
          type="search"
          className="search-input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search messages, stories, comments…"
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

      {results !== null && totalHits === 0 && !busy && (
        <p className="search-hint">no results</p>
      )}

      {results !== null && results.messages.length > 0 && (
        <section className="search-section" data-test="search-messages-section">
          <h3 className="search-section-heading">
            Messages <span className="search-section-count">{results.messages.length}</span>
          </h3>
          <ul className="search-results">
            {results.messages.map((r, i) => (
              <li key={`m-${r.ts}-${i}`} className="search-result">
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
        </section>
      )}

      {results !== null && results.stories.length > 0 && (
        <section className="search-section" data-test="search-stories-section">
          <h3 className="search-section-heading">
            Stories <span className="search-section-count">{results.stories.length}</span>
          </h3>
          <ul className="search-results">
            {results.stories.map((s) => (
              <li
                key={`s-${s.number}`}
                className="search-result search-result--clickable"
                data-test={`search-story-${s.number}`}
              >
                <button
                  type="button"
                  className="search-result-link"
                  onClick={() => handleStoryClick(s.number)}
                >
                  <header className="search-result-meta">
                    <strong className="search-result-story-num">#{s.number}</strong>
                    <span className="search-result-story-title">{s.title}</span>
                  </header>
                  <p className="search-result-text">{s.snippet}</p>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {results !== null && results.comments.length > 0 && (
        <section className="search-section" data-test="search-comments-section">
          <h3 className="search-section-heading">
            Comments <span className="search-section-count">{results.comments.length}</span>
          </h3>
          <ul className="search-results">
            {results.comments.map((c) => (
              <li
                key={`c-${c.comment_id}`}
                className="search-result search-result--clickable"
                data-test={`search-comment-${c.comment_id}`}
              >
                <button
                  type="button"
                  className="search-result-link"
                  onClick={() => handleCommentClick(c.story_number, c.comment_id)}
                >
                  <header className="search-result-meta">
                    <strong className="search-result-story-num">
                      on #{c.story_number}
                    </strong>
                    <cite className="search-result-from">{c.author}</cite>
                  </header>
                  <p className="search-result-text">{c.snippet}</p>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </section>
  )
}

export default Search
