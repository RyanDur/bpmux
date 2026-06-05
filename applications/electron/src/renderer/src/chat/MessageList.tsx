import { useMemo, useState } from 'react'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageClass } from '../hooks/useMessageFilter'
import { MESSAGE_TYPE } from '../transport/messageType'
import { useMentionScoping } from '../hooks/useMentionScoping'
import { usePresence } from '../hooks/usePresence'
import { useScrollGutter } from '../hooks/useScrollGutter'
import { deriveTeamRoles } from '../roles/deriveRoles'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import MessageItem from './MessageItem'
import { messageKey, shortName } from './MessageItem.utils'
import { isMessageInScope } from './scope'
import './MessageList.css'

interface Props {
  messages: ChatMessage[]
  visibleClasses: ReadonlySet<MessageClass>
  reactionsByMessage?: Map<string, import('../hooks/useReactions').AggregatedReaction[]>
  onToggleReaction?: (messageKey: string, emoji: string) => void
  viewer?: string
  apiBase?: string
  authToken?: string
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
  if ((curr.messageType ?? MESSAGE_TYPE.Chat) !== (prev.messageType ?? MESSAGE_TYPE.Chat))
    return false
  if ((curr.addressee ?? null) !== (prev.addressee ?? null)) return false
  const currMs = new Date(curr.ts).getTime()
  const prevMs = new Date(prev.ts).getTime()
  if (Number.isNaN(currMs) || Number.isNaN(prevMs)) return false
  return currMs - prevMs <= GROUP_THRESHOLD_MS
}

const MessageList = ({
  messages,
  visibleClasses,
  reactionsByMessage,
  onToggleReaction,
  viewer,
  apiBase,
  authToken,
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
      visibleClasses.has(classOf(m.messageType)) && isMessageInScope(m, recipient, viewer, allView)
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

  const channelKey = allView ? '__all__' : recipient ? recipient.join(',') : '__none__'
  let currentMaxTs = 0
  for (const m of visible) {
    const ts = new Date(m.ts).getTime()
    if (!Number.isNaN(ts) && ts > currentMaxTs) currentMaxTs = ts
  }
  const [entrySnapshot, setEntrySnapshot] = useState<{
    channelKey: string | null
    newestTs: number
    enteringIds: ReadonlySet<string>
  }>({ channelKey: null, newestTs: 0, enteringIds: new Set() })

  if (entrySnapshot.channelKey !== channelKey) {
    setEntrySnapshot({ channelKey, newestTs: currentMaxTs, enteringIds: new Set() })
  } else if (currentMaxTs > entrySnapshot.newestTs) {
    const fresh = new Set<string>()
    for (const m of visible) {
      const ts = new Date(m.ts).getTime()
      if (!Number.isNaN(ts) && ts > entrySnapshot.newestTs) fresh.add(m.id)
    }
    setEntrySnapshot({ channelKey, newestTs: currentMaxTs, enteringIds: fresh })
  }

  const enteringIds =
    entrySnapshot.channelKey === channelKey ? entrySnapshot.enteringIds : new Set<string>()

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
    const mKey = messageKey(m)
    return (
      <MessageItem
        key={m.id}
        m={m}
        grouped={grouped}
        isNew={enteringIds.has(m.id)}
        viewer={viewer}
        apiBase={apiBase}
        authToken={authToken}
        presence={presence}
        mentionRoles={mentionRoles}
        onMention={handleMention}
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

  const reversed = buildReversed(visible)
  return (
    <section className="message-list-wrap" data-test="message-list-wrap">
      {isLoadingOlder && (
        <output className="loading-older-pill" data-test="loading-older-pill" aria-live="polite">
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
    </section>
  )
}

export default MessageList
