import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { format } from 'date-fns'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import { usePresence } from '../hooks/usePresence'
import { deriveTeamRoles } from '../roles/deriveRoles'
import { roleColorFor } from '../roles/roleColor'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import ClassGlyph from './ClassGlyph'
import MessageBody from './MessageBody'
import Reactions, { ReactionAddButton } from './Reactions'
import UnfurlCards from './UnfurlCards'
import { isMessageInScope } from './scope'
import './MessageList.css'

interface Props {
  messages: ChatMessage[]
  visibleClasses: ReadonlySet<MessageClass>
  pinnedKeys?: ReadonlySet<string>
  onTogglePin?: (m: ChatMessage) => void
  reactionsByMessage?: Map<string, import('../hooks/useReactions').AggregatedReaction[]>
  onToggleReaction?: (messageKey: string, emoji: string) => void
  viewer?: string
  apiBase?: string
  authToken?: string
  lanes?: 'on' | 'off'
  recipient?: string[] | null
  onSelectRecipient?: (r: string[] | null) => void
}

const GROUP_THRESHOLD_MS = 5 * 60 * 1000

const formatTime = (ts: string): string => {
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return format(d, 'h:mm a zzz')
}

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

const scopeLabel = (m: ChatMessage): string | null => {
  if (!m.to || m.to.length === 0) return null
  if (m.to.includes('all')) return null
  return `→ ${m.to.join(', ')}`
}

const isGroupedWithPrev = (curr: ChatMessage, prev: ChatMessage | null): boolean => {
  if (!prev) return false
  if (shortName(curr.from) !== shortName(prev.from)) return false
  if ((curr.messageType ?? 'chat') !== (prev.messageType ?? 'chat')) return false
  if ((curr.addressee ?? null) !== (prev.addressee ?? null)) return false
  const currMs = new Date(curr.ts).getTime()
  const prevMs = new Date(prev.ts).getTime()
  if (Number.isNaN(currMs) || Number.isNaN(prevMs)) return false
  return currMs - prevMs <= GROUP_THRESHOLD_MS
}

const avatarClasses = (sender: string): string => {
  const shape = sender === 'ryan' ? 'message-avatar--circle' : 'message-avatar--square'
  return `message-avatar ${shape}`
}

const pinKey = (m: ChatMessage): string => `${m.ts}|${m.from}`

const MessageList = ({
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
  recipient = null,
  onSelectRecipient
}: Props): React.JSX.Element => {
  const visible = messages.filter(
    (m) => visibleClasses.has(classOf(m.messageType)) && isMessageInScope(m, recipient, viewer)
  )
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
  // #380: @mention pills. The set of valid @-roles is the same
  // runtime-derived team set used for presence — consistent with
  // #393 AC1's "no hardcoded team composition." Wrapped in a Set
  // so MessageBody's per-string O(1) membership check is cheap.
  const mentionRoles = useMemo(() => new Set(presenceRoles), [presenceRoles])
  const handleMention = useCallback(
    (role: string): void => {
      onSelectRecipient?.([role])
    },
    [onSelectRecipient]
  )

  // #388 "new messages ↓" indicator. When the user has scrolled up
  // from the bottom (scrollTop !== 0 under column-reverse layout
  // from #363) and new messages arrive, count how many have
  // accumulated since the scroll-away. Click on the pill scrolls
  // back to bottom + resets the count. Bottom-detection uses a
  // small threshold (8px) so sub-pixel scroll noise doesn't flap
  // the indicator on and off.
  const BOTTOM_THRESHOLD_PX = 8
  const listRef = useRef<HTMLUListElement | null>(null)
  const atBottomRef = useRef<boolean>(true)
  const lastLengthRef = useRef<number>(visible.length)
  const [newCount, setNewCount] = useState<number>(0)

  const handleScroll = useCallback((): void => {
    const ul = listRef.current
    if (!ul) return
    // column-reverse: visual bottom is scrollTop === 0. Negative
    // scrollTop can occur on some browsers; abs keeps the math
    // honest for both engines.
    const atBottom = Math.abs(ul.scrollTop) <= BOTTOM_THRESHOLD_PX
    atBottomRef.current = atBottom
    if (atBottom && newCount !== 0) setNewCount(0)
  }, [newCount])

  useEffect(() => {
    const prev = lastLengthRef.current
    const curr = visible.length
    lastLengthRef.current = curr
    if (curr <= prev) return
    // New messages arrived. If the user is at the bottom, the
    // column-reverse layout scrolls automatically and there's
    // nothing to notify about — the new messages are visible.
    // Otherwise, accumulate the count of arrivals since the last
    // scroll-away.
    if (atBottomRef.current) return
    setNewCount((n) => n + (curr - prev))
  }, [visible.length])

  const scrollToBottom = useCallback((): void => {
    const ul = listRef.current
    if (!ul) return
    // scrollTo(0) lands at visual bottom under column-reverse; the
    // scroll event handler will reset the count once
    // atBottomRef lands true.
    ul.scrollTo({ top: 0, behavior: 'smooth' })
    setNewCount(0)
  }, [])

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
              {!grouped && (() => {
                // #370 sender-info tooltip. Avatar becomes a button so
                // keyboard users can focus + reveal via focus-within;
                // sighted pointer users trigger via :hover with a
                // 150ms transition-delay (Nielsen-timing). Unique
                // per-message id for aria-describedby so duplicate
                // avatars don't collide.
                //
                // #370 rework: tooltip no longer carries a static
                // role-description (previous hardcoded "pm — product
                // backlog…" etc baked the current team composition
                // into the renderer). Shape is now runtime-derived
                // label (sender's short-role, capitalized) + current
                // presence status. Richer self-descriptions become
                // future-work once an agent-driven `get_identity`
                // metadata channel lands.
                const label = sender.charAt(0).toUpperCase() + sender.slice(1)
                const status = presence.get(sender) ?? 'offline'
                const tooltipId = `avatar-info-${m.id}`
                // #401: color comes from the runtime palette; inline
                // style applies per-render so any discovered role
                // gets a distinct color without a CSS token for it.
                const roleColor = roleColorFor(sender)
                return (
                  <span className="message-avatar-wrap">
                    <button
                      type="button"
                      className={avatarClasses(sender)}
                      data-test="message-avatar"
                      data-role={sender}
                      style={{
                        background: roleColor.background,
                        color: roleColor.color
                      }}
                      aria-describedby={tooltipId}
                      aria-label={`${label} — ${status}`}
                    >
                      {sender.charAt(0).toUpperCase()}
                    </button>
                    <span
                      id={tooltipId}
                      role="tooltip"
                      className="avatar-tooltip"
                      data-test="avatar-tooltip"
                    >
                      <strong className="avatar-tooltip-label">{label}</strong>
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
                <MessageBody
                  text={m.text}
                  knownRoles={mentionRoles}
                  onMention={handleMention}
                />
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
    <div className="message-list-wrap" data-test="message-list-wrap">
      <ul {...regionProps} ref={listRef} onScroll={handleScroll}>
        {reversed.map(renderRow)}
      </ul>
      {newCount > 0 && (
        <button
          type="button"
          className="new-messages-pill"
          data-test="new-messages-pill"
          onClick={scrollToBottom}
          role="status"
          aria-live="polite"
        >
          {newCount === 1 ? '1 new message ↓' : `${newCount} new messages ↓`}
        </button>
      )}
    </div>
  )
}

export default MessageList
