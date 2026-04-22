import { format } from 'date-fns'
import { useEffect, useRef } from 'react'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import MessageBody from './MessageBody'
import './MessageList.css'

interface Props {
  messages: ChatMessage[]
  // Classes the user wants to see. Messages outside the set are
  // filtered from the render. Passed as a Set (not a flat array) so
  // membership check is O(1) per message — MessageList can be hot on
  // large logs.
  visibleClasses: ReadonlySet<MessageClass>
}

// Per-class gutter glyph (AC6). Chat = no glyph (majority case, zero
// visual noise). Other classes get a small inline prefix so the
// reader can scan message class without relying on the filter alone.
const CLASS_GLYPHS: Record<MessageClass, string | null> = {
  chat: null,
  insight: '◊',
  activity: '•',
  system: '⚙'
}

// Grouping threshold — consecutive messages from the same sender
// within this window collapse into a single visual group (one avatar
// + one role-name header, subsequent bodies indent under it). Spec's
// default; tunable as a "revisit after live-use" per #348's story
// body. Kept as a named constant so the value is greppable when
// calibration feedback arrives.
const GROUP_THRESHOLD_MS = 5 * 60 * 1000

// Role keys we recognise for avatar coloring + shape differentiation.
// Anything else falls through to 'unknown' (no role-color, neutral
// avatar). Matches --role-* tokens in tokens.css.
const KNOWN_ROLES = new Set(['ryan', 'designer', 'dev', 'pm'])

function formatTime(ts: string): string {
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return format(d, 'h:mm a zzz')
}

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function scopeLabel(m: ChatMessage): string | null {
  if (!m.to || m.to.length === 0) return null
  if (m.to.includes('all')) return null
  return `→ ${m.to.join(', ')}`
}

// Grouping predicate — all four conditions must hold to collapse
// `curr` into the previous message's group: same sender, same
// message-class, same addressee, and within the 5-minute threshold.
// Any mismatch breaks the group so the viewer sees a fresh avatar +
// header (ACs 3, 4 explicitly gate on class and addressee).
function isGroupedWithPrev(curr: ChatMessage, prev: ChatMessage | null): boolean {
  if (!prev) return false
  if (shortName(curr.from) !== shortName(prev.from)) return false
  if ((curr.messageType ?? 'chat') !== (prev.messageType ?? 'chat')) return false
  if ((curr.addressee ?? null) !== (prev.addressee ?? null)) return false
  const currMs = new Date(curr.ts).getTime()
  const prevMs = new Date(prev.ts).getTime()
  if (Number.isNaN(currMs) || Number.isNaN(prevMs)) return false
  return currMs - prevMs <= GROUP_THRESHOLD_MS
}

function avatarClasses(sender: string): string {
  const roleKey = KNOWN_ROLES.has(sender) ? sender : 'unknown'
  // Ryan-as-human-anchor: filled circle (preattentive shape) vs
  // rounded-square for AI roles. Designers with colorblindness or
  // mono-theme displays can still differentiate Ryan from AI at a
  // glance via shape alone.
  const shape =
    sender === 'ryan' ? 'message-avatar--circle' : 'message-avatar--square'
  return `message-avatar message-avatar--${roleKey} ${shape}`
}

function MessageList({ messages, visibleClasses }: Props): React.JSX.Element {
  const endRef = useRef<HTMLLIElement>(null)

  useEffect(() => {
    // Smooth scroll on new arrival (#352 AC5). Chromium respects
    // prefers-reduced-motion for behavior='smooth' and falls back
    // to instant automatically. No explicit media-query check.
    endRef.current?.scrollIntoView({ block: 'end', behavior: 'smooth' })
  }, [messages.length])

  const visible = messages.filter((m) => visibleClasses.has(classOf(m.messageType)))

  const regionProps = {
    className: 'message-list',
    'data-test': 'message-list',
    role: 'log' as const,
    'aria-label': 'Chat messages',
    'aria-live': 'polite' as const
  }

  if (visible.length === 0) {
    return (
      <ul {...regionProps} className="message-list message-list--empty">
        <li className="message-empty">no messages yet</li>
        <li ref={endRef} aria-hidden="true" />
      </ul>
    )
  }

  return (
    <ul {...regionProps}>
      {visible.map((m, i) => {
        const prev = i > 0 ? visible[i - 1] : null
        const grouped = isGroupedWithPrev(m, prev)
        const sender = shortName(m.from)
        const scope = scopeLabel(m)
        const addressee = m.addressee ?? null
        const addressedToViewer = addressee === RENDERER_IDENTITY
        const addressedToOther = addressee !== null && !addressedToViewer

        // Row classes carry the per-message modifiers that CSS keys
        // off: --grouped hides the avatar + role row and dims the
        // timestamp; --to-me paints the left-border accent; --to-other
        // dims the body color via --msg-fg-muted.
        const rowClasses = [
          'message-row',
          grouped ? 'message-row--grouped' : 'message-row--first',
          addressedToViewer ? 'message-row--to-me' : '',
          addressedToOther ? 'message-row--to-other' : ''
        ]
          .filter(Boolean)
          .join(' ')

        return (
          <li key={m.id} className={rowClasses} data-test="message-row">
            <article>
              {/* Screen-reader sender prefix. Every article carries
                * this so SR users navigating message-by-message always
                * hear who's speaking, even in grouped continuations
                * where sighted users see the sender only at the group
                * head. */}
              <span className="sr-only">{sender}:</span>
              {!grouped && (
                <span
                  className={avatarClasses(sender)}
                  data-test="message-avatar"
                  data-role={KNOWN_ROLES.has(sender) ? sender : 'unknown'}
                  aria-hidden="true"
                >
                  {sender.charAt(0).toUpperCase()}
                </span>
              )}
              {!grouped && (
                <header className="message-meta">
                  <cite className="message-from" aria-hidden="true">
                    {sender}
                  </cite>
                  <time className="message-ts" dateTime={m.ts}>
                    {formatTime(m.ts)}
                  </time>
                  {scope && <small className="message-scope">{scope}</small>}
                  {addressee && (
                    <small
                      className="message-addressee"
                      title={`attention: @${addressee}`}
                    >
                      @{addressee}
                    </small>
                  )}
                </header>
              )}
              {grouped && (
                <time className="message-ts message-ts--grouped" dateTime={m.ts}>
                  {formatTime(m.ts)}
                </time>
              )}
              <section className="message-body">
                {CLASS_GLYPHS[classOf(m.messageType)] && (
                  <span
                    className="message-class-glyph"
                    data-test="message-class-glyph"
                    data-class={classOf(m.messageType)}
                    aria-hidden="true"
                  >
                    {CLASS_GLYPHS[classOf(m.messageType)]}
                  </span>
                )}
                <MessageBody text={m.text} />
              </section>
            </article>
          </li>
        )
      })}
      <li ref={endRef} aria-hidden="true" />
    </ul>
  )
}

export default MessageList
