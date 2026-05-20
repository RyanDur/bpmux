import { useMemo } from 'react'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import { useMentionScoping } from '../hooks/useMentionScoping'
import { usePresence } from '../hooks/usePresence'
import { useScrollGutter } from '../hooks/useScrollGutter'
import { deriveTeamRoles } from '../roles/deriveRoles'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import MessageItem, { pinKey, shortName } from './MessageItem'
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
  allView?: boolean
  onSelectRecipient?: (r: string[] | null) => void
  loadOlder?: () => Promise<void>
  isLoadingOlder?: boolean
  hasMoreOlder?: boolean
  hasError?: boolean
  onAtBottomChange?: (atBottom: boolean) => void
}

const GROUP_THRESHOLD_MS = 5 * 60 * 1000

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
  allView = false,
  onSelectRecipient,
  loadOlder,
  isLoadingOlder = false,
  hasMoreOlder = true,
  hasError = false,
  onAtBottomChange
}: Props): React.JSX.Element => {
  const visible = messages.filter(
    (m) =>
      visibleClasses.has(classOf(m.messageType)) &&
      isMessageInScope(m, recipient, viewer, allView)
  )
  const presenceRoles = useMemo(
    () => deriveTeamRoles(messages, viewer ?? RENDERER_IDENTITY),
    [messages, viewer]
  )
  const presence = usePresence(messages, presenceRoles)
  const { mentionRoles, handleMention } = useMentionScoping(presenceRoles, onSelectRecipient)

  const { listRef, handleScroll, newCount, scrollToBottom } = useScrollGutter({
    visibleCount: visible.length,
    loadOlder,
    isLoadingOlder,
    hasMoreOlder,
    onAtBottomChange
  })

  const listProps = {
    className: 'message-list',
    'data-test': 'message-list'
  }
  const logRegionProps = {
    role: 'log' as const,
    'aria-label': 'Chat messages',
    'aria-live': 'polite' as const
  }

  if (visible.length === 0) {
    return (
      <section
        className="message-list message-list--empty"
        data-test="message-list"
        role="log"
        aria-label="Chat messages"
        aria-live="polite"
      >
        <p className="message-empty">no messages yet</p>
      </section>
    )
  }

  const renderRow = ({ m, grouped }: { m: ChatMessage; grouped: boolean }): React.JSX.Element => {
    const pinned = pinnedKeys?.has(pinKey(m)) ?? false
    const mKey = pinKey(m)
    return (
      <MessageItem
        key={m.id}
        m={m}
        grouped={grouped}
        viewer={viewer}
        apiBase={apiBase}
        authToken={authToken}
        presence={presence}
        mentionRoles={mentionRoles}
        onMention={handleMention}
        pinned={pinned}
        onTogglePin={onTogglePin}
        reactions={reactionsByMessage?.get(mKey)}
        onToggleReaction={onToggleReaction}
      />
    )
  }

  const buildReversed = (slice: ChatMessage[]): { m: ChatMessage; grouped: boolean }[] => {
    const withGrouping = slice.map((m, i) => ({
      m,
      grouped: isGroupedWithPrev(m, i > 0 ? slice[i - 1] : null)
    }))
    return [...withGrouping].reverse()
  }

  if (lanes === 'on') {
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
              <section role="log" aria-live="polite" className="message-log-region">
                <ul className="message-list message-list--lane">
                  {rows.map(renderRow)}
                </ul>
              </section>
            </section>
          )
        })}
      </div>
    )
  }

  const reversed = buildReversed(visible)
  return (
    <div className="message-list-wrap" data-test="message-list-wrap">
      {isLoadingOlder && (
        <output
          className="loading-older-pill"
          data-test="loading-older-pill"
          aria-live="polite"
        >
          loading older messages…
        </output>
      )}
      {!isLoadingOlder && hasError && loadOlder && (
        <button
          type="button"
          className="loading-older-pill loading-older-pill--error"
          data-test="loading-older-error"
          onClick={() => {
            void loadOlder()
          }}
          aria-label="retry loading older messages"
        >
          couldn’t load older messages — retry
        </button>
      )}
      <section {...logRegionProps} className="message-log-region">
        <ul {...listProps} ref={listRef} onScroll={handleScroll}>
          {reversed.map(renderRow)}
        </ul>
      </section>
      {newCount > 0 && (
        <button
          type="button"
          className="new-messages-pill"
          data-test="new-messages-pill"
          onClick={scrollToBottom}
          aria-live="polite"
        >
          {newCount === 1 ? '1 new message ↓' : `${newCount} new messages ↓`}
        </button>
      )}
    </div>
  )
}

export default MessageList
