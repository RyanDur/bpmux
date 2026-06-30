import { type Story } from './types'
import { blockerIdFromLabels, deriveBadge, labelFor, type BadgeKind } from './StoryBadge.utils'

const tooltipFor = (kind: BadgeKind, state: Story['state'], blockerId: number | null): string => {
  if (kind === 'blocked') {
    return blockerId !== null
      ? `blocked — waiting on #${blockerId}`
      : 'blocked — (no blocker specified)'
  }
  if (kind === 'rework') return 'rework cycle — awaiting re-delivery'
  if (kind === 'pending')
    return 'delivered — awaiting desk-check verdict (acceptor not yet assigned)'
  const accepted = state === 'accepted'
  if (kind === 'team-accepted') return accepted ? 'accepted by team' : 'awaiting team accept'
  if (kind === 'dev-accepted') return accepted ? 'accepted by dev' : 'awaiting dev accept'
  return accepted ? 'accepted by owner' : 'awaiting owner accept'
}

const BlockerGlyph = (): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true" focusable="false">
    <circle cx="6" cy="6" r="4.5" stroke="currentColor" strokeWidth="1" />
    <line x1="3" y1="9" x2="9" y2="3" stroke="currentColor" strokeWidth="1" strokeLinecap="round" />
  </svg>
)

const KIND_GLYPH: Record<Exclude<BadgeKind, 'blocked'>, string> = {
  'team-accepted': '✓',
  accepted: '✓',
  'dev-accepted': '✓',
  pending: '⋯',
  rework: '↻'
}

interface Props {
  story: Story
  onOpenTimeline?: (storyId: number) => void
  onOpenBlocker?: (storyId: number) => void
}

const StoryBadge = ({ story, onOpenTimeline, onOpenBlocker }: Props): React.JSX.Element | null => {
  const kind = deriveBadge(story)
  if (kind === null) return null
  const blockerId = kind === 'blocked' ? blockerIdFromLabels(story.labels) : null
  const tooltip = tooltipFor(kind, story.state, blockerId)
  const label = labelFor(kind, story.state)
  const classes = `story-badge story-badge--${kind}`

  const handler =
    kind === 'blocked'
      ? onOpenBlocker
        ? () => onOpenBlocker(blockerId ?? story.id)
        : undefined
      : onOpenTimeline
        ? () => onOpenTimeline(story.id)
        : undefined
  const ariaSuffix =
    kind === 'blocked' ? (blockerId !== null ? ' — open blocker' : '') : ' — open timeline'

  const inner = (
    <>
      {kind === 'blocked' ? <BlockerGlyph /> : <i aria-hidden="true">{KIND_GLYPH[kind]}</i>}
      <small>{label}</small>
    </>
  )

  if (handler) {
    return (
      <button
        type="button"
        className={`${classes} story-badge--action`}
        data-test="story-badge"
        data-badge-kind={kind}
        data-state={story.state}
        title={tooltip}
        aria-label={`${label}: ${tooltip}${ariaSuffix}`}
        onClick={(e) => {
          e.stopPropagation()
          handler()
        }}
      >
        {inner}
      </button>
    )
  }
  return (
    <output
      className={classes}
      data-test="story-badge"
      data-badge-kind={kind}
      data-state={story.state}
      title={tooltip}
      aria-label={`${label}: ${tooltip}`}
    >
      {inner}
    </output>
  )
}

export default StoryBadge
