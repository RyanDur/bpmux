import type { MessageClass } from '../hooks/useMessageFilter'

interface GlyphProps {
  label: string
  dataClass: MessageClass
}

const EventsGlyph = ({ label, dataClass }: GlyphProps): React.JSX.Element => (
  <svg
    width="12"
    height="12"
    viewBox="0 0 12 12"
    fill="none"
    role="img"
    aria-label={label}
    data-test="message-class-glyph"
    data-class={dataClass}
    className="message-class-glyph"
  >
    <circle cx="6" cy="6" r="4" stroke="currentColor" strokeWidth="1" opacity="0.5" />
    <circle cx="6" cy="6" r="1.8" fill="currentColor" />
  </svg>
)

const BpmuxGlyph = ({ label, dataClass }: GlyphProps): React.JSX.Element => (
  <svg
    width="12"
    height="12"
    viewBox="0 0 12 12"
    fill="none"
    role="img"
    aria-label={label}
    data-test="message-class-glyph"
    data-class={dataClass}
    className="message-class-glyph"
  >
    <path
      d="M6 1.5l.6 1.1a3.2 3.2 0 0 1 1.4.6l1.2-.4.8.8-.4 1.2a3.2 3.2 0 0 1 .6 1.4l1.1.6-1.1.6a3.2 3.2 0 0 1-.6 1.4l.4 1.2-.8.8-1.2-.4a3.2 3.2 0 0 1-1.4.6L6 10.5l-.6-1.1a3.2 3.2 0 0 1-1.4-.6l-1.2.4-.8-.8.4-1.2a3.2 3.2 0 0 1-.6-1.4L.7 5.2l1.1-.6a3.2 3.2 0 0 1 .6-1.4l-.4-1.2.8-.8 1.2.4a3.2 3.2 0 0 1 1.4-.6L6 1.5z"
      stroke="currentColor"
      strokeWidth="0.8"
      strokeLinejoin="round"
    />
    <circle cx="6" cy="6" r="1.4" stroke="currentColor" strokeWidth="0.8" />
  </svg>
)

interface Props {
  cls: MessageClass
}

const ClassGlyph = ({ cls }: Props): React.JSX.Element | null => {
  if (cls === 'events') return <EventsGlyph label="events" dataClass={cls} />
  if (cls === 'bpmux') return <BpmuxGlyph label="bpmux" dataClass={cls} />
  return null
}

export default ClassGlyph
