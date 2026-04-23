import { useMemo } from 'react'
import { format } from 'date-fns'
import { roleInfoFor } from '../constants/roleInfo'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import { usePresence } from '../hooks/usePresence'
import { deriveTeamRoles } from '../roles/deriveRoles'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import ClassGlyph from './ClassGlyph'
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
  // #374 lane separation mode. 'on' = class-grouped <section>s with
  // sticky headers; 'off' (default) = current interleaved render.
  lanes?: 'on' | 'off'
  // #385 channel scope: drives the per-row data-scope attribute for
  // dim-not-hide rendering. `null` = broadcast view (no DM-scope
  // active); `[role]` = DM view; `[role, role, ...]` = group view.
  // When null, only non-broadcast messages not addressed to viewer
  // get dimmed — broadcasts stay primary.
  recipient?: string[] | null
}

// Per-class gutter glyphs moved to `./ClassGlyph` (#373 — custom
// SVG replaces the unicode ◊/•/⚙ so rendering is consistent across
// OS font stacks). Chat stays null (majority case, zero visual
// noise); ClassGlyph returns null for that case too.

// #385: per-message scope-attribute calculator per designer's spec
// (phase-1b/04-channel-scope-visual.md). Returns 'dim' when the
// message is out-of-scope for the current channel view; null when
// primary (default full opacity). Channel selection shifts scope +
// emphasis, never visibility — "nothing hidden from owner" principle.
//
// Primary when any of:
//   1. message addresses the viewer (`to` includes viewer short-name)
//   2. broadcast message AND viewer is in broadcast view
//   3. message's channel id matches the current view channel id
// else → dim.
function computeMessageChannelId(m: ChatMessage): string {
  const to = m.to
  if (!to || to.length === 0 || to.includes('all')) return 'broadcast'
  const sender = shortName(m.from)
  const participants = new Set<string>([sender, ...to])
  return [...participants].sort().join('+')
}

function computeViewChannelId(recipient: string[] | null, viewer: string): string {
  if (!recipient || recipient.length === 0) return 'broadcast'
  return [viewer, ...recipient].sort().join('+')
}

export function scopeAttributeFor(
  m: ChatMessage,
  recipient: string[] | null | undefined,
  viewer: string | undefined
): 'dim' | null {
  const v = viewer ?? ''
  const r = recipient ?? null
  const to = m.to
  if (to && v && to.includes(v)) return null
  const isBroadcast = !to || to.length === 0 || to.includes('all')
  if (isBroadcast && (r === null || r.length === 0)) return null
  if (computeMessageChannelId(m) === computeViewChannelId(r, v)) return null
  return 'dim'
}

// Grouping threshold — consecutive messages from the same sender
// within this window collapse into a single visual group (one avatar
// + one role-name header, subsequent bodies indent under it). Spec's
// default; tunable as a "revisit after live-use" per #348's story
// body. Kept as a named constant so the value is greppable when
// calibration feedback arrives.
const GROUP_THRESHOLD_MS = 5 * 60 * 1000

// #393 team-shape audit: role set derives from the observed message
// stream (plus viewer) at runtime — no hardcoded team composition.
// Avatar coloring still falls back to the 'unknown' --role-* token
// for senders that lack a CSS palette entry, but any discovered role
// is first-class for role-key assignment + presence tracking.
//
// The palette-inventory list below is NOT a team composition claim
// — it's a bridge to the static `--role-{name}` CSS tokens defined
// in `styles/tokens.css`. When AC2 ships (dynamic palette via hash-
// to-hue function), this list retires entirely; until then any
// role outside the list gets the neutral `--role-unknown` color.
// Named `PALETTE_ROLE_TOKENS` to make the CSS-file-dependency
// explicit in the source.
const PALETTE_ROLE_TOKENS: ReadonlySet<string> = new Set([
  'ryan',
  'designer',
  'dev',
  'pm'
])

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
  const roleKey = PALETTE_ROLE_TOKENS.has(sender) ? sender : 'unknown'
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
  authToken,
  lanes = 'off',
  recipient = null
}: Props): React.JSX.Element {
  const visible = messages.filter((m) => visibleClasses.has(classOf(m.messageType)))
  // #370 avatar-hover tooltip: derive current presence per role from
  // the full message stream (not just the filtered visible set) so
  // activity broadcasts that fall outside the user's current class
  // filter still feed the status indicator. Reused from the sidebar
  // presence derivation path.
  // #393: role set comes from runtime derivation — every agent that
  // has spoken in the session (or the viewer) gets a presence entry.
  // No hardcoded team composition.
  const presenceRoles = useMemo(
    () => deriveTeamRoles(messages, viewer ?? RENDERER_IDENTITY),
    [messages, viewer]
  )
  const presence = usePresence(messages, presenceRoles)

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
  // Shared row-render: takes a list of messages (assumed already
  // chronological within its lane), computes grouping flags, then
  // reverses for the column-reverse layout. Used by both the
  // interleaved single-ul path and the lane-separated multi-ul path.
  const renderRow = ({ m, grouped }: { m: ChatMessage; grouped: boolean }): React.JSX.Element => {
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
        // #385 scope: 'dim' if out-of-current-channel, null if primary.
        // data-attribute pattern mirrors theme / density / lanes —
        // CSS branches off `[data-scope="dim"]` without JSX conditional.
        const channelScope = scopeAttributeFor(m, recipient, viewer)
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
          <li
            key={m.id}
            className={rowClasses}
            data-test="message-row"
            data-scope={channelScope ?? undefined}
          >
            <article>
              {/* Screen-reader sender prefix. Every article carries
                * this so SR users navigating message-by-message always
                * hear who's speaking, even in grouped continuations
                * where sighted users see the sender only at the group
                * head. */}
              <span className="sr-only">{sender}:</span>
              {!grouped && (() => {
                // #370 sender-info tooltip. Avatar becomes a button so
                // keyboard users can focus + reveal via focus-within;
                // sighted pointer users trigger via :hover with a
                // 150ms transition-delay (Nielsen-timing). Tooltip
                // text = role label + description (static) + current
                // presence (derived from activity broadcasts). Unique
                // per-message id for aria-describedby so duplicate
                // avatars don't collide.
                const info = roleInfoFor(sender)
                const status = presence.get(sender) ?? 'offline'
                const tooltipId = `avatar-info-${m.id}`
                return (
                  <span className="message-avatar-wrap">
                    <button
                      type="button"
                      className={avatarClasses(sender)}
                      data-test="message-avatar"
                      data-role={PALETTE_ROLE_TOKENS.has(sender) ? sender : 'unknown'}
                      aria-describedby={tooltipId}
                      aria-label={`${info.label} — ${status}`}
                    >
                      {sender.charAt(0).toUpperCase()}
                    </button>
                    <span
                      id={tooltipId}
                      role="tooltip"
                      className="avatar-tooltip"
                      data-test="avatar-tooltip"
                    >
                      <strong className="avatar-tooltip-label">{info.label}</strong>
                      <em className="avatar-tooltip-desc">{info.description}</em>
                      <small
                        className={`avatar-tooltip-status avatar-tooltip-status--${status}`}
                        data-test="avatar-tooltip-status"
                      >
                        {status}
                      </small>
                    </span>
                  </span>
                )
              })()}
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
                <ClassGlyph cls={classOf(m.messageType)} />
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
  }

  // Build a reversed (newest-first DOM) row list for a chronological
  // slice. Grouping recomputed per slice so a class split doesn't
  // inherit a cross-class "grouped" flag from the interleaved order.
  const buildReversed = (slice: ChatMessage[]): { m: ChatMessage; grouped: boolean }[] => {
    const withGrouping = slice.map((m, i) => ({
      m,
      grouped: isGroupedWithPrev(m, i > 0 ? slice[i - 1] : null)
    }))
    return [...withGrouping].reverse()
  }

  if (lanes === 'on') {
    // Lane mode (#374): bucket by message class, render each
    // non-empty class as its own <section> with a sticky header.
    // Order: chat first (primary reader focus + largest section),
    // then insight / activity / system below. System lives last
    // because it's typically the lowest-volume + least reader-
    // priority class.
    const LANE_ORDER: MessageClass[] = ['chat', 'insight', 'activity', 'system']
    const byClass = new Map<MessageClass, ChatMessage[]>()
    for (const cls of LANE_ORDER) byClass.set(cls, [])
    for (const m of visible) {
      const cls = classOf(m.messageType)
      byClass.get(cls)?.push(m)
    }
    return (
      <div className="message-lanes" data-test="message-lanes">
        {LANE_ORDER.map((cls) => {
          const slice = byClass.get(cls) ?? []
          if (slice.length === 0) return null
          const rows = buildReversed(slice)
          return (
            <section
              key={cls}
              className={`message-lane message-lane--${cls}`}
              data-test="message-lane"
              data-lane-class={cls}
              aria-label={`${cls} messages`}
            >
              <header className="message-lane-header">
                <h3 className="message-lane-title">{cls}</h3>
                <small className="message-lane-count">{slice.length}</small>
              </header>
              <ul
                className="message-list message-list--lane"
                role="log"
                aria-live="polite"
              >
                {rows.map(renderRow)}
              </ul>
            </section>
          )
        })}
      </div>
    )
  }

  // Default interleaved render. Single <ul> with chronological
  // grouping computed across all visible messages, then reversed
  // for column-reverse layout.
  const reversed = buildReversed(visible)
  return (
    <>
      <ul {...regionProps}>{reversed.map(renderRow)}</ul>
    </>
  )
}

export default MessageList
