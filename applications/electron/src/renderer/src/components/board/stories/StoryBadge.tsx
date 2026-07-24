import { type Story } from '../types'
import { deriveBadge, labelFor, type BadgeKind } from './StoryBadge.utils'

const tooltipFor = (kind: BadgeKind, state: Story['state']): string => {
  if (kind === 'rework') return 'rework cycle — awaiting re-delivery'
  if (kind === 'pending')
    return 'delivered — awaiting desk-check verdict (acceptor not yet assigned)'
  const accepted = state === 'accepted'
  if (kind === 'team-accepted') return accepted ? 'accepted by team' : 'awaiting team accept'
  if (kind === 'dev-accepted') return accepted ? 'accepted by dev' : 'awaiting dev accept'
  return accepted ? 'accepted by owner' : 'awaiting owner accept'
}

const KIND_GLYPH: Record<BadgeKind, string> = {
  'team-accepted': '✓',
  accepted: '✓',
  'dev-accepted': '✓',
  pending: '⋯',
  rework: '↻'
}

type Props = {
  story: Story
  onOpenTimeline?: (storyId: number) => void
}

const StoryBadge = ({ story, onOpenTimeline }: Props): React.JSX.Element | null => {
  const kind = deriveBadge(story)
  if (kind === null) return null
  const tooltip = tooltipFor(kind, story.state)
  const label = labelFor(kind, story.state)
  const classes = `story-badge story-badge--${kind}`

  const handler = onOpenTimeline ? () => onOpenTimeline(story.id) : undefined
  const ariaSuffix = ' — open timeline'

  const inner = (
    <>
      <i aria-hidden="true">{KIND_GLYPH[kind]}</i>
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
