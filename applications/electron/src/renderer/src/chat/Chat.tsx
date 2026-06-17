import { useMemo } from 'react'
import { classOf, useMessageFilter, type MessageClass } from '../hooks/useMessageFilter'
import { useReactions } from '../hooks/useReactions'
import Metrics from '../metrics/Metrics'
import { RENDERER_IDENTITY, type SendOptions } from '../transport/useWebSocket'
import { useConnectionStatus } from '../transport/session'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import FilterPills from './FilterPills'
import Input from './Input'
import MessageList from './MessageList'
import { TerminalView } from './TerminalView'
import type { TerminalEntry } from '../hooks/useTerminals'
import './Chat.css'

interface Props {
  messages: ChatMessage[]
  recipient: string[] | null
  allView: boolean
  activeKey: string | null
  terminals: TerminalEntry[]
  onCloseTerminal?: (key: string) => void
  onRestartTerminal?: (key: string) => void
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
  mirrorRoster?: readonly RosterMemberWire[]
}

const channelKey = (recipient: string[] | null): string => {
  if (!recipient || recipient.length === 0) return 'broadcast'
  return [...recipient].sort().join('+')
}

const Chat = ({
  messages,
  recipient,
  allView,
  activeKey,
  terminals,
  onCloseTerminal = () => {},
  onRestartTerminal = () => {},
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
  mirrorRoster
}: Props): React.JSX.Element => {
  const status = useConnectionStatus()
  const showReconnect = status !== 'connected'
  const reactionsChannel = channelKey(recipient)
  const reactions = useReactions({
    apiBase,
    authToken,
    channel: reactionsChannel
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

  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <output className="chat-warning" aria-live="polite" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </output>
      )}
      <section className="chat-center">
        {terminals.map((t) => (
          <section
            key={t.key}
            className="terminal-layer"
            data-active={activeKey === t.key}
            data-test={`terminal-slot-${t.key}`}
          >
            {t.state === 'ready' && t.id ? (
              <TerminalView
                terminalId={t.id}
                cwd={t.cwd ?? ''}
                isActive={activeKey === t.key}
                onClose={() => onCloseTerminal(t.key)}
              />
            ) : (
              <section
                className="terminal-state"
                data-test={`terminal-state-${t.key}`}
                data-state={t.state}
              >
                <p className="terminal-state-label">
                  {t.state === 'spawning'
                    ? 'spawning…'
                    : t.state === 'spawn-failed'
                      ? `spawn-failed: ${t.error ?? ''}`
                      : 'exited'}
                </p>
                {(t.state === 'exited' || t.state === 'spawn-failed') && (
                  <footer className="terminal-state-actions">
                    <button
                      type="button"
                      className="terminal-state-action"
                      data-test={`terminal-restart-${t.key}`}
                      onClick={() => onRestartTerminal(t.key)}
                    >
                      {t.state === 'exited' ? 'restart' : 'retry'}
                    </button>
                    <button
                      type="button"
                      className="terminal-state-action terminal-state-action--close"
                      data-test={`terminal-pane-close-${t.key}`}
                      onClick={() => onCloseTerminal(t.key)}
                    >
                      close
                    </button>
                  </footer>
                )}
              </section>
            )}
          </section>
        ))}
        <section className="chat-layer" data-active={activeKey === null}>
          <FilterPills filter={filter} hiddenCounts={hiddenCounts} visibleCounts={visibleCounts} />
          <MessageList
            messages={messages}
            visibleClasses={filter.visible}
            reactionsByMessage={reactions.byMessage}
            onToggleReaction={(key, emoji) => reactions.toggle(key, emoji, RENDERER_IDENTITY)}
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
            mirrorRoster={mirrorRoster}
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
        </section>
      </section>
    </section>
  )
}

export default Chat
