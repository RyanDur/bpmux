import type { ChatMessage, ConnectionStatus } from '../transport/types'
import Input from './Input'
import MessageList from './MessageList'
import './Chat.css'

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  onSend: (text: string, addressee?: string | null) => void
}

function Chat({ messages, status, onSend }: Props): React.JSX.Element {
  const showReconnect = status !== 'connected'
  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <aside className="chat-warning" role="status" aria-live="polite" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </aside>
      )}
      <MessageList messages={messages} />
      <Input onSend={onSend} />
    </section>
  )
}

export default Chat
