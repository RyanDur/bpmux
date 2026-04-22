import { format } from 'date-fns'
import { useRef } from 'react'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import MessageBody from './MessageBody'
import Reactions, { ReactionAddButton } from './Reactions'
import UnfurlCards from './UnfurlCards'
import './MessageList.css'

interface Props {
  messages: ChatMessage[]
  // Classes the user wants to see. Messages outside the set are
  // filtered from the render. Passed as a Set (not a flat array) so
  // membership check is O(1) per message — MessageList can be hot on
  // large logs.
  visibleClasses: ReadonlySet<MessageClass>
  // Pin integration (#342). pinnedKeys tells each row whether to show
  // the pinned indicator; onTogglePin fires the pin/unpin intent for
  // the click handler. Both optional so callers that don't care
  // (e.g., tests) can render without wiring pins.
  pinnedKeys?: ReadonlySet<string>
  onTogglePin?: (m: ChatMessage) => void
  // Reactions integration (#340). reactionsByMessage lookup by
  // message_key returns the per-emoji pills for the row (or
  // undefined if none). onToggleReaction fires add/remove intent
  // from a pill or picker click.
  reactionsByMessage?: Map<string, import('../hooks/useReactions').AggregatedReaction[]>
  onToggleReaction?: (messageKey: string, emoji: string) => void
  viewer?: string
  // Unfurl integration (#343). UnfurlCards fetches its own data
  // server-side; MessageList just forwards the apiBase/authToken
  // so each row's URL set gets resolved. Optional — callers that
  // don't care (tests) omit.
  apiBase?: string
  authToken?: string
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

// Stable per-message key for pin lookup. Mirrors the Rust server's
// session::pins::{ts}|{from} key and the usePins hook's messageKey().
function pinKey(m: ChatMessage): string {
  return `${m.ts}|${m.from}`
}

function MessageList({
  messages,
  visibleClasses,
  pinnedKeys,
  onTogglePin,
  reactionsByMessage,
  onToggleReaction,
  viewer,
  apiBase,
  authToken
}: Props): React.JSX.Element {
  // Hidden sentinel kept so the zero-message empty-state render path
  // below still compiles without structural change. The auto-scroll
  // behaviour itself is handled by CSS column-reverse (#363) — no JS
  // scrolling needed.
  const endRef = useRef<HTMLLIElement>(null)

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

  // #363 scroll preservation: rendered reverse-chronological (newest
  // first in DOM) with CSS `flex-direction: column-reverse` flipping
  // the visual order back to chronological (oldest top, newest
  // bottom). The browser's natural scroll behavior under column-
  // reverse keeps the user anchored to their current scroll position
  // when new messages arrive — no JS scrollIntoView needed. Grouping
  // logic still needs chronological iteration to detect
  // "consecutive-same-sender," so we compute group flags in
  // chronological order first, then reverse the result.
  const withGrouping = visible.map((m, i) => ({
    m,
    grouped: isGroupedWithPrev(m, i > 0 ? visible[i - 1] : null)
  }))
  const reversed = [...withGrouping].reverse()

  return (
    <>
      <ul {...regionProps}>
        {reversed.map(({ m, grouped }) => {
          const sender = shortName(m.from)
          const scope = scopeLabel(m)
          const addressee = m.addressee ?? null
          const addressedToViewer = addressee === RENDERER_IDENTITY
          const addressedToOther = addressee !== null && !addressedToViewer

        // Row classes carry the per-message modifiers that CSS keys
        // off: --grouped hides the avatar + role row and dims the
        // timestamp; --to-me paints the left-border accent; --to-other
        // dims the body color via --msg-fg-muted.
        const pinned = pinnedKeys?.has(pinKey(m)) ?? false
        const rowClasses = [
          'message-row',
          grouped ? 'message-row--grouped' : 'message-row--first',
          addressedToViewer ? 'message-row--to-me' : '',
          addressedToOther ? 'message-row--to-other' : '',
          pinned ? 'message-row--pinned' : ''
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
                  {pinned && (
                    <span
                      className="message-pinned-indicator"
                      data-test="message-pinned-indicator"
                      aria-label="pinned"
                      title="pinned in this channel"
                    >
                      📌
                    </span>
                  )}
                  {onTogglePin && (
                    <button
                      type="button"
                      className="message-pin-toggle"
                      data-test="message-pin-toggle"
                      aria-label={pinned ? 'Unpin message' : 'Pin message'}
                      title={pinned ? 'Unpin' : 'Pin'}
                      onClick={() => onTogglePin(m)}
                    >
                      {pinned ? '✖' : '📌'}
                    </button>
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
                <UnfurlCards text={m.text} apiBase={apiBase} authToken={authToken} />
                {(() => {
                  if (!onToggleReaction || !viewer) return null
                  const mKey = pinKey(m)
                  const rxns = reactionsByMessage?.get(mKey)
                  if (rxns && rxns.length > 0) {
                    return (
                      <Reactions
                        reactions={rxns}
                        viewer={viewer}
                        onToggle={(emoji) => onToggleReaction(mKey, emoji)}
                      />
                    )
                  }
                  // No existing reactions — show compact add-only
                  // affordance that hover-reveals on the row.
                  return (
                    <ReactionAddButton
                      onToggle={(emoji) => onToggleReaction(mKey, emoji)}
                    />
                  )
                })()}
              </section>
            </article>
          </li>
        )
      })}
        <li ref={endRef} aria-hidden="true" />
      </ul>
    </>
  )
}

export default MessageList
