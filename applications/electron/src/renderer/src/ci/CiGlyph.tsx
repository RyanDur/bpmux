import { format } from 'date-fns'
import { useCallback, useEffect, useRef, useState } from 'react'
import { CI_STATE_UPDATE_MARKER, type ChatMessage } from '../transport/types'
import './CiGlyph.css'

interface CiStateReport {
  conclusion: 'success' | 'failure' | 'in_progress' | 'unknown'
  last_run_id: number | null
  branch: string
  url: string
  head_sha: string | null
  display_title: string | null
}

// Safety-net poll cadence. #361: 60s was too slow — a typical fast
// CI run (<60s) could start + finish between polls. 15s catches
// transient states with substantial headroom and stays well under
// GitHub's 5000/hr authenticated rate limit (240 polls/hr).
//
// #372: primary refresh is now the CI_STATE_UPDATE_MARKER broadcast
// from the backend watcher — state changes land within WS latency,
// not up to 15s. The poll stays as a safety net so a disconnect-
// then-reconnect window can't strand the glyph on a conclusion that
// changed while WS was down.
const POLL_MS = 15_000

function conclusionGlyph(conclusion: string): string {
  switch (conclusion) {
    case 'success':
      return '✓'
    case 'failure':
      return '✗'
    case 'in_progress':
      return '◌'
    default:
      return '?'
  }
}

interface Props {
  // WS message stream forwarded from useWebSocket. CiGlyph watches
  // for CI_STATE_UPDATE_MARKER and refetches on receipt. Omit
  // (undefined) when the caller doesn't have a stream — the glyph
  // falls back to poll-only.
  messages?: ChatMessage[]
}

function CiGlyph({ messages }: Props): React.JSX.Element {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const [state, setState] = useState<CiStateReport | null>(null)
  const [error, setError] = useState<string | null>(null)
  // #361: surface the last-successful-fetch timestamp in the tooltip
  // so the user can confirm the poll is alive even when CI is stably
  // green.
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)
  // #372: only react to NEW marker arrivals, not the hydration
  // backfill on mount. Same seenMessageCountRef pattern as pins /
  // reactions / timeline.
  const seenMessageCountRef = useRef<number>(messages?.length ?? 0)
  // Keep a stable reference to the fetch fn so the marker-watch
  // effect can call it without re-running when messages mutate.
  const fetchOnceRef = useRef<() => Promise<void>>(async () => {})

  const buildFetch = useCallback(
    (signal: AbortSignal, cancelledCheck: () => boolean) => async (): Promise<void> => {
      if (!bridge?.apiBase) return
      try {
        const res = await fetch(`${bridge.apiBase}/api/ci`, {
          headers: { Authorization: `Bearer ${bridge.authToken ?? ''}` },
          signal
        })
        if (!res.ok) {
          if (!cancelledCheck()) setError(`HTTP ${res.status}`)
          return
        }
        const json = (await res.json()) as CiStateReport
        if (!cancelledCheck()) {
          setState(json)
          setError(null)
          setLastUpdated(new Date())
        }
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
        if (!cancelledCheck()) setError(err instanceof Error ? err.message : String(err))
      }
    },
    [bridge?.apiBase, bridge?.authToken]
  )

  useEffect(() => {
    if (!bridge?.apiBase) {
      setError('no backend')
      return
    }
    let cancelled = false
    const controller = new AbortController()
    const fetchOnce = buildFetch(controller.signal, () => cancelled)
    fetchOnceRef.current = fetchOnce
    void fetchOnce()
    const id = setInterval(() => {
      void fetchOnce()
    }, POLL_MS)
    return () => {
      cancelled = true
      controller.abort()
      clearInterval(id)
    }
  }, [bridge?.apiBase, buildFetch])

  // #372 push-based update: on each new message batch, check for
  // CI_STATE_UPDATE_MARKER and refetch. seenMessageCountRef ensures
  // we ignore the hydration backfill.
  useEffect(() => {
    if (!messages) return
    const prev = seenMessageCountRef.current
    seenMessageCountRef.current = messages.length
    if (messages.length <= prev) return
    const newMessages = messages.slice(prev)
    const hasCiUpdate = newMessages.some((m) => m.text === CI_STATE_UPDATE_MARKER)
    if (hasCiUpdate) void fetchOnceRef.current()
  }, [messages])

  const conclusion = state?.conclusion ?? 'unknown'
  const glyph = conclusionGlyph(conclusion)
  const url = state?.url ?? 'https://github.com'
  const tooltip = buildTooltip(state, error, lastUpdated)

  return (
    <a
      className={`ci-glyph ci-glyph--${conclusion}`}
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => {
        e.preventDefault()
        if (url) window.open(url, '_blank', 'noopener,noreferrer')
      }}
      title={tooltip}
      aria-label={`CI ${conclusion}`}
      data-test="ci-glyph"
    >
      CI {glyph}
    </a>
  )
}

function buildTooltip(
  state: CiStateReport | null,
  error: string | null,
  lastUpdated: Date | null
): string {
  if (error) return `CI error: ${error}`
  if (!state) return 'CI state loading…'
  const lines = [`CI · ${state.branch} · ${state.conclusion}`]
  if (state.display_title) lines.push(state.display_title)
  if (state.head_sha) lines.push(state.head_sha.slice(0, 7))
  if (lastUpdated) lines.push(`checked ${format(lastUpdated, 'h:mm:ss a')}`)
  lines.push('(click to open on GitHub)')
  return lines.join('\n')
}

export default CiGlyph
