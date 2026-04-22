import { useMemo } from 'react'
import { classOf, useMessageFilter, type MessageClass } from '../hooks/useMessageFilter'
import Metrics from '../metrics/Metrics'
import Search from '../search/Search'
import type { ChatMessage, ConnectionStatus } from '../transport/types'
import type { SendOptions } from '../transport/useWebSocket'
import FilterPills from './FilterPills'
import Input from './Input'
import MessageList from './MessageList'
import './Chat.css'

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  recipient: string | null
  searchOpen: boolean
  metricsOpen: boolean
  onCloseSearch: () => void
  onCloseMetrics: () => void
  apiBase: string | undefined
  authToken: string | undefined
  onSend: (text: string, opts?: SendOptions) => void
}

function Chat({
  messages,
  status,
  recipient,
  searchOpen,
  metricsOpen,
  onCloseSearch,
  onCloseMetrics,
  apiBase,
  authToken,
  onSend
}: Props): React.JSX.Element {
  const showReconnect = status !== 'connected'
  const filter = useMessageFilter()

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
    if (recipient) opts.to = [recipient]
    onSend(text, opts)
  }

  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <aside className="chat-warning" role="status" aria-live="polite" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </aside>
      )}
      {recipient && (
        <aside className="chat-scope" aria-live="polite">
          <small>DM → {recipient}</small>
        </aside>
      )}
      <FilterPills filter={filter} hiddenCounts={hiddenCounts} />
      {metricsOpen ? (
        <Metrics apiBase={apiBase} authToken={authToken} onClose={onCloseMetrics} />
      ) : searchOpen ? (
        <Search apiBase={apiBase} authToken={authToken} onClose={onCloseSearch} />
      ) : (
        <MessageList messages={messages} visibleClasses={filter.visible} />
      )}
      <Input onSend={handleSend} placeholder={recipient ? `message @${recipient}` : undefined} />
    </section>
  )
}

export default Chat
