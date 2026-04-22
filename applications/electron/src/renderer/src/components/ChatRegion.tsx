import type { ChatMessage, ConnectionStatus } from '../types'
import ChatInput from './ChatInput'
import MessageList from './MessageList'

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  onSend: (text: string) => void
}

function ChatRegion({ messages, status, onSend }: Props): React.JSX.Element {
  const showReconnect = status !== 'connected'
  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <div className="chat-warning" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </div>
      )}
      <MessageList messages={messages} />
      <ChatInput onSend={onSend} />
    </section>
  )
}

export default ChatRegion
