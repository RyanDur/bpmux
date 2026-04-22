import { storyTypeFromLabels, type BoardStory } from './types'

// #368: team-accept audit badge. Three variants:
//   - rework: story is currently in a desk-check-fail cycle (amber)
//   - team-accepted: story is in the team-gate lane (bug/chore),
//     either awaiting PM accept (delivered state) or already accepted
//     by the team (accepted state). Solid green.
//   - accepted: story is in the owner-gate lane (feature), either
//     awaiting Ryan's accept (delivered) or already accepted by Ryan.
//     Outlined green — shape-distinct from solid so colorblind
//     readers differentiate lanes, too.
//
// Badge lane is TYPE-DRIVEN (bug/chore = team, feature = owner) per
// the 3-type delegation vocabulary (#264). Rework overrides both
// lanes because the active cycle is the immediate signal.

export type BadgeKind = 'team-accepted' | 'accepted' | 'rework'

const DESK_CHECK_FAIL_LABEL = 'desk-check-fail'

// Decide which badge (if any) should surface on a row. Only stories
// in delivered / accepted states get a badge — upstream states
// (ready/in-progress/icebox) don't carry accept semantics yet.
export function deriveBadge(story: BoardStory): BadgeKind | null {
  if (story.state !== 'delivered' && story.state !== 'accepted') return null
  const hasRework = story.labels.some(
    (l) => l.toLowerCase() === DESK_CHECK_FAIL_LABEL
  )
  if (hasRework) return 'rework'
  const type = storyTypeFromLabels(story.labels)
  if (type === 'bug' || type === 'chore') return 'team-accepted'
  // features default to owner-accept lane; unknown type defaults
  // there too (conservative — owner-accept is the stricter gate).
  return 'accepted'
}

// Human-readable tooltip + aria-label per kind and current state.
// Stays short so the badge remains visually unobtrusive; the
// drill-in timeline (#367) carries the full audit trail.
function tooltipFor(kind: BadgeKind, state: BoardStory['state']): string {
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

// Glyph per kind. ✓ for accept variants (shape difference via CSS
// --solid vs --outlined), ↻ for rework (distinct glyph doubles the
// color signal for a11y).
const KIND_GLYPH: Record<BadgeKind, string> = {
  'team-accepted': '✓',
  accepted: '✓',
  rework: '↻'
}

const KIND_LABEL: Record<BadgeKind, string> = {
  'team-accepted': 'team-accepted',
  accepted: 'accepted',
  rework: 'rework'
}

interface Props {
  story: BoardStory
  // When set, clicking the badge asks the board to open the drill-in
  // with the timeline section auto-expanded. Omit if the row itself
  // already handles the open + the badge is decorative.
  onOpenTimeline?: (storyNumber: number) => void
}

function StoryBadge({ story, onOpenTimeline }: Props): React.JSX.Element | null {
  const kind = deriveBadge(story)
  if (kind === null) return null
  const tooltip = tooltipFor(kind, story.state)
  const label = KIND_LABEL[kind]
  const glyph = KIND_GLYPH[kind]
  const classes = `story-badge story-badge--${kind}`
  // If the row provides an onOpenTimeline handler, render as a button
  // so keyboard + screen reader users can activate it; otherwise
  // render as a plain span — decorative only. Either way the
  // tooltip / aria-label communicates the accept lane.
  if (onOpenTimeline) {
    return (
      <button
        type="button"
        className={classes}
        data-test="story-badge"
        data-badge-kind={kind}
        title={tooltip}
        aria-label={`${label}: ${tooltip} — open timeline`}
        onClick={(e) => {
          // Prevent the row's open-drill-in handler from firing a
          // second time when the user clicks the badge specifically.
          e.stopPropagation()
          onOpenTimeline(story.number)
        }}
      >
        <span aria-hidden="true">{glyph}</span>
        <small>{label}</small>
      </button>
    )
  }
  return (
    <span
      className={classes}
      data-test="story-badge"
      data-badge-kind={kind}
      title={tooltip}
      aria-label={`${label}: ${tooltip}`}
    >
      <span aria-hidden="true">{glyph}</span>
      <small>{label}</small>
    </span>
  )
}

export default StoryBadge
