import { useEffect, useMemo, useState } from 'react'
import { classOf, useMessageFilter, type MessageClass } from '../hooks/useMessageFilter'
import { channelKey, messageKey, usePins } from '../hooks/usePins'
import { useReactions } from '../hooks/useReactions'
import Metrics from '../metrics/Metrics'
import { RENDERER_IDENTITY, type SendOptions } from '../transport/useWebSocket'
import type {
  ChatMessage,
  ConnectionStatus,
  PinnedMessageWire,
  ReactionAggregateWire
} from '../transport/types'
import FilterPills from './FilterPills'
import Input from './Input'
import MessageList from './MessageList'
import PinnedList from './PinnedList'
import './Chat.css'

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  recipient: string[] | null
  allView: boolean
  onSelectRecipient?: (r: string[] | null) => void
  filter: ReturnType<typeof useMessageFilter>
  metricsOpen: boolean
  onCloseMetrics: () => void
  apiBase: string | undefined
  authToken: string | undefined
  onSend: (text: string, opts?: SendOptions) => void
  loadOlder?: () => Promise<void>
  isLoadingOlder?: boolean
  hasMoreOlder?: boolean
  hasError?: boolean
  onAtBottomChange?: (atBottom: boolean) => void
  onPinnedKeysChange?: (keys: ReadonlySet<string>) => void
  mirrorReactions?: ReadonlyMap<string, ReactionAggregateWire[]>
  mirrorPins?: ReadonlyMap<string, PinnedMessageWire[]>
}

const Chat = ({
  messages,
  status,
  recipient,
  allView,
  onSelectRecipient,
  filter,
  metricsOpen,
  onCloseMetrics,
  apiBase,
  authToken,
  onSend,
  loadOlder,
  isLoadingOlder,
  hasMoreOlder,
  hasError,
  onAtBottomChange,
  onPinnedKeysChange,
  mirrorReactions,
  mirrorPins
}: Props): React.JSX.Element => {
  const showReconnect = status !== 'connected'
  const [pinnedListOpen, setPinnedListOpen] = useState<boolean>(false)
  const pinsChannel = channelKey(recipient)
  const pins = usePins({ apiBase, authToken, channel: pinsChannel, mirrorPins })

  useEffect(() => {
    onPinnedKeysChange?.(pins.pinnedKeys)
  }, [pins.pinnedKeys, onPinnedKeysChange])

  const reactions = useReactions({
    apiBase,
    authToken,
    channel: pinsChannel,
    mirrorReactions
  })

  const togglePin = (m: ChatMessage): void => {
    const key = messageKey(m)
    if (pins.pinnedKeys.has(key)) {
      void pins.unpin(key)
    } else {
      void pins.pin({ from: m.from, ts: m.ts, text: m.text }, RENDERER_IDENTITY)
    }
  }

  const hiddenCounts = useMemo((): Partial<Record<MessageClass, number>> => {
    const counts: Partial<Record<MessageClass, number>> = {}
    for (const m of messages) {
      const cls = classOf(m.messageType)
      if (filter.visible.has(cls)) continue
      counts[cls] = (counts[cls] ?? 0) + 1
    }
    return counts
  }, [messages, filter.visible])

  const handleSend = (text: string, addressee: string | null): void => {
    const opts: SendOptions = { addressee }
    if (recipient && recipient.length > 0) opts.to = [...recipient]
    onSend(text, opts)
  }

  const scopeLabel = allView
    ? 'ALL CONVERSATIONS'
    : recipient && recipient.length > 0
      ? `${recipient.length === 1 ? 'DM' : 'GROUP'} → ${recipient.join(' + ')}`
      : null

  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <output className="chat-warning" aria-live="polite" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </output>
      )}
      {scopeLabel && (
        <aside className="chat-scope" aria-live="polite">
          <small>{scopeLabel}</small>
        </aside>
      )}
      <aside
        className="chat-channel-header"
        data-test="chat-channel-header"
        aria-label="Channel actions"
      >
        <button
          type="button"
          className="chat-pin-count"
          data-test="chat-pin-count"
          onClick={() => setPinnedListOpen(true)}
          disabled={pins.pins.length === 0}
          aria-label={`${pins.pins.length} pinned messages — open list`}
          title="View pinned messages"
        >
          📌 <span>{pins.pins.length}</span>
        </button>
      </aside>
      <FilterPills filter={filter} hiddenCounts={hiddenCounts} />
      <MessageList
        messages={messages}
        visibleClasses={filter.visible}
        pinnedKeys={pins.pinnedKeys}
        onTogglePin={togglePin}
        reactionsByMessage={reactions.byMessage}
        onToggleReaction={(key, emoji) => {
          void reactions.toggle(key, emoji, RENDERER_IDENTITY)
        }}
        viewer={RENDERER_IDENTITY}
        recipient={recipient}
        allView={allView}
        onSelectRecipient={onSelectRecipient}
        apiBase={apiBase}
        authToken={authToken}
        loadOlder={loadOlder}
        isLoadingOlder={isLoadingOlder}
        hasMoreOlder={hasMoreOlder}
        hasError={hasError}
        onAtBottomChange={onAtBottomChange}
      />
      <Metrics
        apiBase={apiBase}
        authToken={authToken}
        onClose={onCloseMetrics}
        open={metricsOpen}
      />
      <PinnedList
        pins={pins.pins}
        onUnpin={(key) => {
          void pins.unpin(key)
        }}
        onClose={() => setPinnedListOpen(false)}
        open={pinnedListOpen}
      />
      <Input
        key={allView ? 'all' : pinsChannel}
        onSend={handleSend}
        channelKey={allView ? 'all' : pinsChannel}
        placeholder={
          recipient && recipient.length > 0 ? `message ${recipient.join(' + ')}` : undefined
        }
      />
    </section>
  )
}

export default Chat
