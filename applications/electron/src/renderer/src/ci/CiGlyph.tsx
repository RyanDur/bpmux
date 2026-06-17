import { format } from 'date-fns'
import { useCallback, useEffect, useState } from 'react'
import { describeHttpError, http } from '../api/http'
import type { CiSnapshotWire, CiStateWire } from '../transport/types'
import {
  derivedClassName,
  derivedDataState,
  isWatcherUnhealthy,
  watcherUnhealthyAriaLabel
} from './watcherHealth'
import './CiGlyph.css'

type CiConclusion = 'success' | 'failure' | 'in_progress' | 'quota_exhausted' | 'unknown'

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

const conclusionGlyph = (conclusion: string): string => {
  switch (conclusion) {
    case 'success':
      return '✓'
    case 'failure':
      return '✗'
    case 'in_progress':
      return '◌'
    case 'quota_exhausted':
      return '⏸'
    default:
      return '?'
  }
}

const conclusionAriaLabel = (conclusion: string): string => {
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

const CiGlyph = ({ ciSnapshot }: Props): React.JSX.Element => {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const [state, setState] = useState<CiStateReport | null>(null)
  const [fetchError, setFetchError] = useState<string | null>(null)
  const error = bridge?.apiBase ? fetchError : 'no backend'
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)

  const buildFetch = useCallback(
    (signal: AbortSignal, cancelledCheck: () => boolean) => (): void => {
      if (!bridge?.apiBase) return
      http
        .get<CiStateReport>(
          { apiBase: bridge.apiBase, authToken: bridge.authToken },
          '/api/ci',
          signal
        )
        .onSuccess((json) => {
          if (cancelledCheck()) return
          setState(json)
          setFetchError(null)
          setLastUpdated(new Date())
        })
        .onFailure((err) => {
          if (cancelledCheck()) return
          setFetchError(describeHttpError(err))
        })
    },
    [bridge?.apiBase, bridge?.authToken]
  )

  useEffect(() => {
    if (!bridge?.apiBase) return
    let cancelled = false
    const controller = new AbortController()
    const fetchOnce = buildFetch(controller.signal, () => cancelled)
    fetchOnce()
    const id = setInterval(() => {
      fetchOnce()
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
        watcherUnhealthy ? watcherUnhealthyAriaLabel(state) : conclusionAriaLabel(conclusion)
      }
      data-test="ci-glyph"
      data-conclusion={conclusion}
      data-state={derivedDataState(conclusion, watcherUnhealthy)}
    >
      CI {glyph}
    </a>
  )
}

const buildTooltip = (
  state: CiStateReport | null,
  error: string | null,
  lastUpdated: Date | null,
  watcherUnhealthy: boolean
): string => {
  if (error) return `CI error: ${error}`
  if (!state) return 'CI state loading…'
  const lines = [`CI · ${state.branch} · ${state.conclusion}`]
  if (state.display_title) lines.push(state.display_title)
  if (state.head_sha) lines.push(state.head_sha.slice(0, 7))
  if (lastUpdated) lines.push(`checked ${format(lastUpdated, 'h:mm:ss a')}`)
  if (watcherUnhealthy && state.last_successful_poll_at) {
    const ageSecs = Math.round((Date.now() - Date.parse(state.last_successful_poll_at)) / 1000)
    lines.push(`⚠ watcher last polled ${ageSecs}s ago`)
  }
  lines.push('(click to open on GitHub)')
  return lines.join('\n')
}

export default CiGlyph
