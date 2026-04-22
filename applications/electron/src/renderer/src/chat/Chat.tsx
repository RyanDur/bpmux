import type { ChatMessage, ConnectionStatus } from '../transport/types'
import Input from './Input'
import MessageList from './MessageList'
import './Chat.css'

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  onSend: (text: string) => void
}

function Chat({ messages, status, onSend }: Props): React.JSX.Element {
  const showReconnect = status !== 'connected'
  return (
    <section className="chat" data-test="chat-region">
      {showReconnect && (
        <div className="chat-warning" data-test="chat-warning">
          {status === 'connecting' ? 'connecting…' : 'reconnecting…'}
        </div>
      )}
      <MessageList messages={messages} />
      <Input onSend={onSend} />
    </section>
  )
}

export default Chat
