import { storyTypeFromLabels, type BoardStory } from './types'

// #368: team-accept audit badge. Variants:
//   - blocked (#390): story has a `blocked` label while in delivered
//     — "not actionable for you right now." Highest precedence on
//     delivered rows so owner's acceptance walk can skip it at a
//     glance. Inline SVG road-sign glyph + amber family.
//   - rework: story is currently in a desk-check-fail cycle (red)
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
// lanes because the active cycle is the immediate signal. Blocked
// overrides rework because "not in your queue" outranks "needs a
// redelivery cycle" as an owner-facing skip signal.

export type BadgeKind = 'team-accepted' | 'accepted' | 'rework' | 'blocked'

const DESK_CHECK_FAIL_LABEL = 'desk-check-fail'
const BLOCKED_LABEL = 'blocked'
const BLOCKED_BY_PREFIX = 'blocked-by:'

// Parse `blocked-by:<N>` label (case-insensitive) into a story number.
// Returns null when absent or malformed — the caller falls back to
// a generic "no blocker specified" tooltip rather than hiding the
// block-state signal entirely (#390 nothing-hidden principle).
export function blockerNumberFromLabels(labels: string[]): number | null {
  for (const l of labels) {
    const lower = l.toLowerCase()
    if (lower.startsWith(BLOCKED_BY_PREFIX)) {
      const n = Number.parseInt(lower.slice(BLOCKED_BY_PREFIX.length), 10)
      if (Number.isFinite(n) && n > 0) return n
    }
  }
  return null
}

// Decide which badge (if any) should surface on a row. Only stories
// in delivered / accepted states get a badge — upstream states
// (ready/in-progress/icebox) don't carry accept semantics yet.
export function deriveBadge(story: BoardStory): BadgeKind | null {
  if (story.state !== 'delivered' && story.state !== 'accepted') return null
  const normalized = story.labels.map((l) => l.toLowerCase())
  // Blocked takes precedence on delivered rows only. Accepted rows
  // shouldn't carry block-state; if one does (data-shape gap), the
  // accept-lane badge wins — the story is done, block is stale.
  if (story.state === 'delivered' && normalized.includes(BLOCKED_LABEL)) {
    return 'blocked'
  }
  const hasRework = normalized.includes(DESK_CHECK_FAIL_LABEL)
  if (hasRework) return 'rework'
  const type = storyTypeFromLabels(story.labels)
  if (type === 'bug' || type === 'chore') return 'team-accepted'
  // features default to owner-accept lane; unknown type defaults
  // there too (conservative — owner-accept is the stricter gate).
  return 'accepted'
}

// State-aware label text. Kind identifies the lane (owner vs team
// accept lane; rework override; blocked override). State disambiguates
// tense on accept variants; rework + blocked are transient states
// that only make sense on delivered rows.
export function labelFor(kind: BadgeKind, state: BoardStory['state']): string {
  if (kind === 'blocked') return 'blocked'
  if (kind === 'rework') return 'rework'
  if (state === 'accepted') {
    return kind === 'team-accepted' ? 'team-accepted' : 'owner-accepted'
  }
  // state === 'delivered'
  return kind === 'team-accepted' ? 'awaiting team' : 'awaiting owner'
}

// Human-readable tooltip + aria-label per kind and current state.
// Stays short so the badge remains visually unobtrusive; the
// drill-in timeline (#367) carries the full audit trail.
function tooltipFor(
  kind: BadgeKind,
  state: BoardStory['state'],
  blockerNumber: number | null
): string {
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
function BlockerGlyph(): React.JSX.Element {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      <circle cx="6" cy="6" r="4.5" stroke="currentColor" strokeWidth="1" />
      <line
        x1="3"
        y1="9"
        x2="9"
        y2="3"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinecap="round"
      />
    </svg>
  )
}

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

function StoryBadge({
  story,
  onOpenTimeline,
  onOpenBlocker
}: Props): React.JSX.Element | null {
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
    kind === 'blocked'
      ? blockerNumber !== null
        ? ' — open blocker'
        : ''
      : ' — open timeline'

  const inner = (
    <>
      {kind === 'blocked' ? (
        <BlockerGlyph />
      ) : (
        <span aria-hidden="true">{KIND_GLYPH[kind]}</span>
      )}
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
        className={classes}
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
    <span
      className={classes}
      data-test="story-badge"
      data-badge-kind={kind}
      data-state={story.state}
      title={tooltip}
      aria-label={`${label}: ${tooltip}`}
    >
      {inner}
    </span>
  )
}

export default StoryBadge
