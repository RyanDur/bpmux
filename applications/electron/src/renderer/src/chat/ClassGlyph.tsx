import type { MessageClass } from '../hooks/useMessageFilter'

interface Props {
  cls: MessageClass
}

const EventsGlyph = (): React.JSX.Element => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true" focusable="false">
    <circle cx="6" cy="6" r="4" stroke="currentColor" strokeWidth="1" opacity="0.5" />
    <circle cx="6" cy="6" r="1.8" fill="currentColor" />
  </svg>
)

const BpmuxGlyph = (): React.JSX.Element => (
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
  events: 'events',
  bpmux: 'bpmux'
}

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
      {cls === 'events' && <EventsGlyph />}
      {cls === 'bpmux' && <BpmuxGlyph />}
    </span>
  )
}

export default ClassGlyph
