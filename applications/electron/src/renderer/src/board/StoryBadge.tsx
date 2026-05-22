import { type BoardStory } from './types'
import { blockerNumberFromLabels, deriveBadge, labelFor, type BadgeKind } from './StoryBadge.utils'

// Human-readable tooltip + aria-label per kind and current state.
// Stays short so the badge remains visually unobtrusive; the
// drill-in timeline (#367) carries the full audit trail.
const tooltipFor = (
  kind: BadgeKind,
  state: BoardStory['state'],
  blockerNumber: number | null
): string => {
  if (kind === 'blocked') {
    return blockerNumber !== null
      ? `blocked — waiting on #${blockerNumber}`
      : 'blocked — (no blocker specified)'
  }
  if (kind === 'rework') return 'rework cycle — awaiting re-delivery'
  if (kind === 'team-accepted') {
    return state === 'accepted'
      ? 'accepted by team (bug/chore lane)'
      : 'awaiting team accept (bug/chore lane)'
  }
  return state === 'accepted'
    ? 'accepted by owner (feature lane)'
    : 'awaiting owner accept (feature lane)'
}

// Inline SVG glyph for the blocked badge. Follows the ClassGlyph
// hand-drawn pattern (12x12 viewBox, currentColor) so the block
// indicator renders identically across OS font stacks — the unicode
// ⛔ varies widely on macOS vs Linux vs Windows. Shape is the
// universal road-sign "no-entry" vocabulary: circle + diagonal slash.
// Paired with the glyph text fallback for terminals and SR users
// (aria-hidden on the SVG; the button / span carries the label).
const BlockerGlyph = (): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true" focusable="false">
    <circle cx="6" cy="6" r="4.5" stroke="currentColor" strokeWidth="1" />
    <line x1="3" y1="9" x2="9" y2="3" stroke="currentColor" strokeWidth="1" strokeLinecap="round" />
  </svg>
)

// Text-glyph per kind — used for the non-blocked variants. Blocked
// renders the inline SVG above; keeping it out of this table avoids
// a string-vs-element type split across the record.
const KIND_GLYPH: Record<Exclude<BadgeKind, 'blocked'>, string> = {
  'team-accepted': '✓',
  accepted: '✓',
  rework: '↻'
}

interface Props {
  story: BoardStory
  // When set, clicking an accept-lane badge asks the board to open
  // the drill-in with the timeline section auto-expanded. Decorative
  // otherwise.
  onOpenTimeline?: (storyNumber: number) => void
  // When set AND the badge is the blocked variant, clicking opens
  // the blocker story's drill-in (the story `blocked-by:<N>` points
  // at). Falls back to the own-story drill-in when the `blocked-by:`
  // label is absent so the badge stays clickable.
  onOpenBlocker?: (storyNumber: number) => void
}

const StoryBadge = ({ story, onOpenTimeline, onOpenBlocker }: Props): React.JSX.Element | null => {
  const kind = deriveBadge(story)
  if (kind === null) return null
  const blockerNumber = kind === 'blocked' ? blockerNumberFromLabels(story.labels) : null
  const tooltip = tooltipFor(kind, story.state, blockerNumber)
  // #375: visible label is state-aware.
  const label = labelFor(kind, story.state)
  const classes = `story-badge story-badge--${kind}`

  // Pick the action handler + aria suffix based on kind:
  //   - blocked: onOpenBlocker (opens the blocker drill-in; falls
  //     back to own drill-in when blocker-by is absent)
  //   - others: onOpenTimeline (opens own drill-in with timeline)
  // The button is rendered only when a handler exists and there is
  // a target to open; decorative span otherwise.
  const handler =
    kind === 'blocked'
      ? onOpenBlocker
        ? () => onOpenBlocker(blockerNumber ?? story.number)
        : undefined
      : onOpenTimeline
        ? () => onOpenTimeline(story.number)
        : undefined
  const ariaSuffix =
    kind === 'blocked' ? (blockerNumber !== null ? ' — open blocker' : '') : ' — open timeline'

  const inner = (
    <>
      {kind === 'blocked' ? <BlockerGlyph /> : <i aria-hidden="true">{KIND_GLYPH[kind]}</i>}
      <small>{label}</small>
    </>
  )

  // #376 data-state: CSS branches the color family off this attribute
  // so awaiting rows read amber and accepted rows read green. Blocked
  // variant has its own color family in Board.css.
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
          // Prevent the row's open-drill-in handler from firing a
          // second time when the user clicks the badge specifically.
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
