import { format } from 'date-fns'
import { useCiSnapshot } from '../state'
import type { CiSnapshotWire, CiStateWire } from '../transport/types'
import {
  derivedClassName,
  derivedDataState,
  isWatcherUnhealthy,
  watcherUnhealthyAriaLabel
} from './watcherHealth'
import './CiGlyph.css'

type CiConclusion = 'success' | 'failure' | 'in_progress' | 'quota_exhausted' | 'unknown'

const GITHUB_FALLBACK_URL = 'https://github.com'
const CI_BRANCH = 'main'

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

const CiGlyph = (): React.JSX.Element => {
  const ciSnapshot = useCiSnapshot()
  const actionsUrl = ciSnapshot?.url ?? GITHUB_FALLBACK_URL
  const conclusion = ciSnapshot ? stateToConclusion(ciSnapshot.state) : 'unknown'
  const watcherUnhealthy = isWatcherUnhealthy(ciSnapshot)
  const glyph = conclusionGlyph(conclusion)
  const tooltip = buildTooltip(ciSnapshot, conclusion, watcherUnhealthy)

  return (
    <a
      className={derivedClassName(conclusion, watcherUnhealthy)}
      href={actionsUrl}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => {
        e.preventDefault()
        window.open(actionsUrl, '_blank', 'noopener,noreferrer')
      }}
      title={tooltip}
      aria-label={
        watcherUnhealthy ? watcherUnhealthyAriaLabel(ciSnapshot) : conclusionAriaLabel(conclusion)
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
  ci: CiSnapshotWire | null,
  conclusion: CiConclusion,
  watcherUnhealthy: boolean
): string => {
  if (!ci) return 'CI state loading…'
  const lines = [`CI · ${CI_BRANCH} · ${conclusion}`]
  if (ci.display_title) lines.push(ci.display_title)
  if (ci.head_sha) lines.push(ci.head_sha.slice(0, 7))
  if (ci.last_successful_poll_at) {
    lines.push(`polled ${format(new Date(ci.last_successful_poll_at), 'h:mm:ss a')}`)
  }
  if (watcherUnhealthy && ci.last_successful_poll_at) {
    const ageSecs = Math.round((Date.now() - Date.parse(ci.last_successful_poll_at)) / 1000)
    lines.push(`⚠ watcher last polled ${ageSecs}s ago`)
  }
  lines.push('(click to open on GitHub)')
  return lines.join('\n')
}

export default CiGlyph
