import Search from '../search/Search'
import type { ChatMessage, ConnectionStatus } from '../transport/types'
import type { SendOptions } from '../transport/useWebSocket'
import Input from './Input'
import MessageList from './MessageList'
import './Chat.css'

interface Props {
  messages: ChatMessage[]
  status: ConnectionStatus
  recipient: string | null
  searchOpen: boolean
  onCloseSearch: () => void
  apiBase: string | undefined
  authToken: string | undefined
  onSend: (text: string, opts?: SendOptions) => void
}

function Chat({
  messages,
  status,
  recipient,
  searchOpen,
  onCloseSearch,
  apiBase,
  authToken,
  onSend
}: Props): React.JSX.Element {
  const showReconnect = status !== 'connected'

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
      {searchOpen ? (
        <Search apiBase={apiBase} authToken={authToken} onClose={onCloseSearch} />
      ) : (
        <MessageList messages={messages} />
      )}
      <Input onSend={handleSend} placeholder={recipient ? `message @${recipient}` : undefined} />
    </section>
  )
}

export default Chat
