import { format } from 'date-fns'
import { maybe } from '@ryandur/sand'
import { useCiSnapshot } from '@state'
import type { CiSnapshotWire, CiStateWire } from '@transport/types'
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
  const actionsUrl = ciSnapshot.mBind((snapshot) => maybe(snapshot.url)).orElse(GITHUB_FALLBACK_URL)
  const conclusion = ciSnapshot
    .map((snapshot) => stateToConclusion(snapshot.state))
    .orElse('unknown')
  const watcherUnhealthy = isWatcherUnhealthy(ciSnapshot)
  const glyph = conclusionGlyph(conclusion)
  const tooltip = ciSnapshot
    .map((snapshot) => presentTooltip(snapshot, conclusion, watcherUnhealthy))
    .orElse('CI state loading…')

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

const presentTooltip = (
  ci: CiSnapshotWire,
  conclusion: CiConclusion,
  watcherUnhealthy: boolean
): string =>
  [
    `CI · ${CI_BRANCH} · ${conclusion}`,
    ...(ci.display_title ? [ci.display_title] : []),
    ...(ci.head_sha ? [ci.head_sha.slice(0, 7)] : []),
    ...(ci.last_successful_poll_at
      ? [`polled ${format(new Date(ci.last_successful_poll_at), 'h:mm:ss a')}`]
      : []),
    ...(watcherUnhealthy && ci.last_successful_poll_at
      ? [
          `⚠ watcher last polled ${Math.round((Date.now() - Date.parse(ci.last_successful_poll_at)) / 1000)}s ago`
        ]
      : []),
    '(click to open on GitHub)'
  ].join('\n')

export default CiGlyph
