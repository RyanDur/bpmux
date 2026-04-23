import { useMemo, useState } from 'react'
import { classOf, useMessageFilter, type MessageClass } from '../hooks/useMessageFilter'
import { channelKey, messageKey, usePins } from '../hooks/usePins'
import { useReactions } from '../hooks/useReactions'
import { useAnimatedUnmount } from '../hooks/useAnimatedUnmount'
import Metrics from '../metrics/Metrics'
import Search from '../search/Search'
import { RENDERER_IDENTITY, type SendOptions } from '../transport/useWebSocket'
import type { ChatMessage, ConnectionStatus } from '../transport/types'
import FilterPills from './FilterPills'
import Input from './Input'
import MessageList from './MessageList'
import PinnedList from './PinnedList'
import './Chat.css'


// CSS transition durations that match the overlay CSS. Kept here
// (rather than read from computed style) so the JS-side mount-delay
// doesn't pull from the renderer at every open/close. Must stay in
// sync with --motion-duration-medium in tokens.css.
const OVERLAY_EXIT_MS = 250

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  recipient: string[] | null
  // #380: click-to-scope on inline @mention pills routes through
  // this setter (same shape as the sidebar's recipient selector
  // per #332 click-parity AC).
  onSelectRecipient?: (r: string[] | null) => void
  filter: ReturnType<typeof useMessageFilter>
  searchOpen: boolean
  metricsOpen: boolean
  onCloseSearch: () => void
  onCloseMetrics: () => void
  // Request Board to open drill-in for a story (optionally with a
  // specific comment highlighted). Forwards from search-result clicks.
  onOpenStory: (storyNumber: number, commentId?: number) => void
  apiBase: string | undefined
  authToken: string | undefined
  // #374 lane-separation mode, threaded from AppShell. 'on' =
  // class-grouped sections; 'off' = interleaved (default). Threaded
  // rather than hook-called-twice so AppShell's toggle is the sole
  // source of truth and MessageList re-renders on the same tick.
  lanes?: 'on' | 'off'
  onSend: (text: string, opts?: SendOptions) => void
}

function Chat({
  messages,
  status,
  recipient,
  onSelectRecipient,
  filter,
  searchOpen,
  metricsOpen,
  onCloseSearch,
  onCloseMetrics,
  onOpenStory,
  apiBase,
  authToken,
  lanes = 'off',
  onSend
}: Props): React.JSX.Element {
  const showReconnect = status !== 'connected'
  const [pinnedListOpen, setPinnedListOpen] = useState<boolean>(false)
  const pinsChannel = channelKey(recipient)
  const pins = usePins({ apiBase, authToken, channel: pinsChannel, messages })
  const reactions = useReactions({
    apiBase,
    authToken,
    channel: pinsChannel,
    messages
  })
  const search = useAnimatedUnmount(searchOpen, OVERLAY_EXIT_MS)
  const metrics = useAnimatedUnmount(metricsOpen, OVERLAY_EXIT_MS)
  const pinnedList = useAnimatedUnmount(pinnedListOpen, OVERLAY_EXIT_MS)


  // Pin handler for MessageList. Given an already-rendered message,
  // toggle pin status — either pin (if not already pinned) or unpin.
  const togglePin = (m: ChatMessage): void => {
    const key = messageKey(m)
    if (pins.pinnedKeys.has(key)) {
      void pins.unpin(key)
    } else {
      void pins.pin({ from: m.from, ts: m.ts, text: m.text }, RENDERER_IDENTITY)
    }
  }

  // Hidden-count: for each filtered-out class, how many messages are
  // being kept out of the stream right now. Computed once per render
  // rather than inside FilterPills so the same reduction doesn't fire
  // twice (FilterPills doesn't need the messages array).
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

  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <aside className="chat-warning" role="status" aria-live="polite" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </aside>
      )}
      {recipient && recipient.length > 0 && (
        <aside className="chat-scope" aria-live="polite">
          <small>
            {recipient.length === 1 ? 'DM' : 'GROUP'} → {recipient.join(' + ')}
          </small>
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
      {/* MessageList stays mounted; overlays stack on top when open,
        * using useAnimatedUnmount for close-direction transitions
        * (#354). Each overlay's CSS reads `[data-visible]` to decide
        * between enter-state (visible) and exit-state (closing).
        * Absolute positioning means they share the viewport space
        * with MessageList without layout shift.
        */}
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
        onSelectRecipient={onSelectRecipient}
        apiBase={apiBase}
        authToken={authToken}
        lanes={lanes}
      />
      {metrics.mounted && (
        <Metrics
          apiBase={apiBase}
          authToken={authToken}
          onClose={onCloseMetrics}
          visible={metrics.visible}
        />
      )}
      {search.mounted && (
        <Search
          apiBase={apiBase}
          authToken={authToken}
          onClose={onCloseSearch}
          onOpenStory={onOpenStory}
          visible={search.visible}
        />
      )}
      {pinnedList.mounted && (
        <PinnedList
          pins={pins.pins}
          onUnpin={(key) => {
            void pins.unpin(key)
          }}
          onClose={() => setPinnedListOpen(false)}
          visible={pinnedList.visible}
        />
      )}
      <Input
        onSend={handleSend}
        placeholder={
          recipient && recipient.length > 0
            ? `message ${recipient.join(' + ')}`
            : undefined
        }
      />
    </section>
  )
}

export default Chat
