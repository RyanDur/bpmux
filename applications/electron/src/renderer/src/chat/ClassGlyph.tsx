import type { MessageClass } from '../hooks/useMessageFilter'

// #373: inline SVG glyphs replacing the unicode ◊/•/⚙ from #349.
// Custom-drawn so rendering is consistent across OS font stacks
// (unicode box / symbol glyphs vary widely) and aligns with the
// hand-drawn-icons discipline the CI glyph + theme toggle already
// follow. No external icon lib.
//
// All icons drawn in a 12x12 viewBox so the AC3 floor (min 12px)
// is the literal rendered size. Color defaults to `currentColor`
// so the parent `.message-class-glyph` class controls the tier
// (currently --fg-dim per #349); aria-hidden because the class is
// already announced via the addressee/scope chips in the header.

interface Props {
  cls: MessageClass
}

const InsightGlyph = (): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true" focusable="false">
    <path
      d="M6 1.5a3.2 3.2 0 0 0-2 5.6v1.2h4V7.1A3.2 3.2 0 0 0 6 1.5z"
      stroke="currentColor"
      strokeWidth="1"
      strokeLinejoin="round"
    />
    <line
      x1="4.5"
      y1="9.5"
      x2="7.5"
      y2="9.5"
      stroke="currentColor"
      strokeWidth="1"
      strokeLinecap="round"
    />
    <line
      x1="5"
      y1="11"
      x2="7"
      y2="11"
      stroke="currentColor"
      strokeWidth="1"
      strokeLinecap="round"
    />
  </svg>
)

const ActivityGlyph = (): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true" focusable="false">
    <circle cx="6" cy="6" r="4" stroke="currentColor" strokeWidth="1" opacity="0.5" />
    <circle cx="6" cy="6" r="1.8" fill="currentColor" />
  </svg>
)

const SystemGlyph = (): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true" focusable="false">
    <path
      d="M6 1.5l.6 1.1a3.2 3.2 0 0 1 1.4.6l1.2-.4.8.8-.4 1.2a3.2 3.2 0 0 1 .6 1.4l1.1.6-1.1.6a3.2 3.2 0 0 1-.6 1.4l.4 1.2-.8.8-1.2-.4a3.2 3.2 0 0 1-1.4.6L6 10.5l-.6-1.1a3.2 3.2 0 0 1-1.4-.6l-1.2.4-.8-.8.4-1.2a3.2 3.2 0 0 1-.6-1.4L.7 5.2l1.1-.6a3.2 3.2 0 0 1 .6-1.4l-.4-1.2.8-.8 1.2.4a3.2 3.2 0 0 1 1.4-.6L6 1.5z"
      stroke="currentColor"
      strokeWidth="0.8"
      strokeLinejoin="round"
    />
    <circle cx="6" cy="6" r="1.4" stroke="currentColor" strokeWidth="0.8" />
  </svg>
)

const LABEL: Record<Exclude<MessageClass, 'chat'>, string> = {
  insight: 'insight',
  activity: 'activity',
  system: 'system'
}

// Returns null for `chat` so the majority case adds zero DOM. The
// caller's existing `CLASS_GLYPHS[cls] && <ClassGlyph ...>` gate is
// preserved by null-checking here too for belt-and-suspenders.
const ClassGlyph = ({ cls }: Props): React.JSX.Element | null => {
  if (cls === 'chat') return null
  const label = LABEL[cls]
  return (
    <span
      className="message-class-glyph"
      data-test="message-class-glyph"
      data-class={cls}
      role="img"
      aria-label={label}
    >
      {cls === 'insight' && <InsightGlyph />}
      {cls === 'activity' && <ActivityGlyph />}
      {cls === 'system' && <SystemGlyph />}
    </span>
  )
}

export default ClassGlyph
