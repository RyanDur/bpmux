import { useMemo } from 'react'
import { classOf, useMessageFilter, type MessageClass } from '../hooks/useMessageFilter'
import { useReactions } from '../hooks/useReactions'
import Metrics from '../metrics/Metrics'
import { RENDERER_IDENTITY, type SendOptions } from '../transport/useWebSocket'
import type { ChatMessage, ConnectionStatus, ReactionAggregateWire } from '../transport/types'
import FilterPills from './FilterPills'
import Input from './Input'
import MessageList from './MessageList'
import { TerminalView } from './TerminalView'
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
  mirrorReactions?: ReadonlyMap<string, ReactionAggregateWire[]>
}

const channelKey = (recipient: string[] | null): string => {
  if (!recipient || recipient.length === 0) return 'broadcast'
  return [...recipient].sort().join('+')
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
  mirrorReactions
}: Props): React.JSX.Element => {
  const showReconnect = status !== 'connected'
  const reactionsChannel = channelKey(recipient)
  const reactions = useReactions({
    apiBase,
    authToken,
    channel: reactionsChannel,
    mirrorReactions
  })

  const { hiddenCounts, visibleCounts } = useMemo((): {
    hiddenCounts: Partial<Record<MessageClass, number>>
    visibleCounts: Partial<Record<MessageClass, number>>
  } => {
    const hidden: Partial<Record<MessageClass, number>> = {}
    const visible: Partial<Record<MessageClass, number>> = {}
    for (const m of messages) {
      const cls = classOf(m.messageType)
      if (filter.visible.has(cls)) {
        visible[cls] = (visible[cls] ?? 0) + 1
      } else {
        hidden[cls] = (hidden[cls] ?? 0) + 1
      }
    }
    return { hiddenCounts: hidden, visibleCounts: visible }
  }, [messages, filter.visible])

  const handleSend = (text: string, addressee: string | null): void => {
    const opts: SendOptions = { addressee }
    if (recipient && recipient.length > 0) opts.to = [...recipient]
    onSend(text, opts)
  }

  const scopeLabel = allView
    ? 'ALL CONVERSATIONS'
    : recipient && recipient.length > 1
      ? `GROUP → ${recipient.join(' + ')}`
      : null

  const terminalRole = !allView && recipient && recipient.length === 1 ? recipient[0] : null

  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <output className="chat-warning" aria-live="polite" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </output>
      )}
      {terminalRole ? (
        <TerminalView role={terminalRole} />
      ) : (
        <>
          {scopeLabel && (
            <aside className="chat-scope" aria-live="polite">
              <small>{scopeLabel}</small>
            </aside>
          )}
          <FilterPills filter={filter} hiddenCounts={hiddenCounts} visibleCounts={visibleCounts} />
          <MessageList
            messages={messages}
            visibleClasses={filter.visible}
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
          <Input
            key={allView ? 'all' : reactionsChannel}
            onSend={handleSend}
            channelKey={allView ? 'all' : reactionsChannel}
            placeholder={
              recipient && recipient.length > 0 ? `message ${recipient.join(' + ')}` : undefined
            }
          />
        </>
      )}
    </section>
  )
}

export default Chat
