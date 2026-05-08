import { format } from 'date-fns'
import { useCallback, useEffect, useState } from 'react'
import type { CiSnapshotWire, CiStateWire } from '../transport/types'
import {
  derivedClassName,
  derivedDataState,
  isWatcherUnhealthy,
  watcherUnhealthyAriaLabel
} from './watcherHealth'
import './CiGlyph.css'

// #397 + #361 bundled: `quota_exhausted` is a distinct backend
// conclusion (not a kind of failure) so team-gate code can branch
// between hard-block and pass-with-citation. The glyph renders it
// with ⏸ + amber so the owner can see "infra unavailable" at a
// glance instead of reading it as a code-failure red.
type CiConclusion =
  | 'success'
  | 'failure'
  | 'in_progress'
  | 'quota_exhausted'
  | 'unknown'

interface CiStateReport {
  conclusion: CiConclusion
  last_run_id: number | null
  branch: string
  url: string
  head_sha: string | null
  display_title: string | null
  last_successful_poll_at?: string | null
  poll_interval_secs?: number
  backoff_cap_secs?: number
}

// Safety-net poll cadence. #361: 60s was too slow — a typical fast
// CI run (<60s) could start + finish between polls. 15s catches
// transient states with substantial headroom and stays well under
// GitHub's 5000/hr authenticated rate limit (240 polls/hr).
const DISCONNECT_RECONNECT_SAFETY_POLL_MS = 15_000

const stateToConclusion = (state: CiStateWire): CiConclusion => {
  switch (state) {
    case 'green':
      return 'success'
    case 'red':
      return 'failure'
    case 'running':
      return 'in_progress'
    case 'quota_exhausted':
      return 'quota_exhausted'
    case 'unknown':
      return 'unknown'
  }
}

function conclusionGlyph(conclusion: string): string {
  switch (conclusion) {
    case 'success':
      return '✓'
    case 'failure':
      return '✗'
    case 'in_progress':
      return '◌'
    case 'quota_exhausted':
      // #397 + #361: distinct from ✗ so owner reads "infra paused"
      // not "code failed." Shape carries the signal for color-blind
      // readers; color (amber) reinforces.
      return '⏸'
    default:
      return '?'
  }
}

// Human-readable conclusion for aria-label. Extends the glyph
// vocabulary with accessibility text — `quota_exhausted` needs a
// spelled-out reason so screen-reader users aren't left guessing
// what ⏸ means.
function conclusionAriaLabel(conclusion: string): string {
  switch (conclusion) {
    case 'quota_exhausted':
      return 'CI unavailable — GitHub Actions quota exhausted'
    default:
      return `CI ${conclusion}`
  }
}

interface Props {
  ciSnapshot?: CiSnapshotWire | null
}

function CiGlyph({ ciSnapshot }: Props): React.JSX.Element {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const [state, setState] = useState<CiStateReport | null>(null)
  const [error, setError] = useState<string | null>(null)
  // #361: surface the last-successful-fetch timestamp in the tooltip
  // so the user can confirm the poll is alive even when CI is stably
  // green.
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)

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
    void fetchOnce()
    const id = setInterval(() => {
      void fetchOnce()
    }, DISCONNECT_RECONNECT_SAFETY_POLL_MS)
    return () => {
      cancelled = true
      controller.abort()
      clearInterval(id)
    }
  }, [bridge?.apiBase, buildFetch])

  const conclusion = ciSnapshot
    ? stateToConclusion(ciSnapshot.state)
    : (state?.conclusion ?? 'unknown')
  const watcherUnhealthy = isWatcherUnhealthy(state)
  const glyph = conclusionGlyph(conclusion)
  const url = state?.url ?? 'https://github.com'
  const tooltip = buildTooltip(state, error, lastUpdated, watcherUnhealthy)

  return (
    <a
      className={derivedClassName(conclusion, watcherUnhealthy)}
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => {
        e.preventDefault()
        if (url) window.open(url, '_blank', 'noopener,noreferrer')
      }}
      title={tooltip}
      aria-label={
        watcherUnhealthy
          ? watcherUnhealthyAriaLabel(state)
          : conclusionAriaLabel(conclusion)
      }
      data-test="ci-glyph"
      data-conclusion={conclusion}
      data-state={derivedDataState(conclusion, watcherUnhealthy)}
    >
      CI {glyph}
    </a>
  )
}

function buildTooltip(
  state: CiStateReport | null,
  error: string | null,
  lastUpdated: Date | null,
  watcherUnhealthy: boolean
): string {
  if (error) return `CI error: ${error}`
  if (!state) return 'CI state loading…'
  const lines = [`CI · ${state.branch} · ${state.conclusion}`]
  if (state.display_title) lines.push(state.display_title)
  if (state.head_sha) lines.push(state.head_sha.slice(0, 7))
  if (lastUpdated) lines.push(`checked ${format(lastUpdated, 'h:mm:ss a')}`)
  if (watcherUnhealthy && state.last_successful_poll_at) {
    const ageSecs = Math.round(
      (Date.now() - Date.parse(state.last_successful_poll_at)) / 1000
    )
    lines.push(`⚠ watcher last polled ${ageSecs}s ago`)
  }
  lines.push('(click to open on GitHub)')
  return lines.join('\n')
}

export default CiGlyph
