import Board from './board/Board'
import Chat from './chat/Chat'
import CiGlyph from './ci/CiGlyph'
import Sidebar from './sidebar/Sidebar'
import type { ChatMessage, ConnectionStatus } from './transport/types'
import './AppShell.css'

interface Props {
  status: ConnectionStatus
  messages: ChatMessage[]
  onSend: (text: string) => void
}

// Phase-1a window chrome: titlebar (app name, connection dot, CI
// glyph) on top; sidebar / chat / board regions below. Connection
// status surfaces as a small dot so a backend drop is visible without
// having to read the chat warning strip.
function AppShell({ status, messages, onSend }: Props): React.JSX.Element {
  const statusLabel =
    status === 'connected' ? 'connected' : status === 'connecting' ? 'connecting' : 'disconnected'

  return (
    <div className="app" data-test="app-root">
      <header className="titlebar" data-test="titlebar">
        <span className="titlebar-app">bpmux</span>
        <span className="titlebar-spacer" />
        <span
          className={`titlebar-status titlebar-status--${status}`}
          title={statusLabel}
          aria-label={`connection ${statusLabel}`}
        />
        <CiGlyph />
      </header>
      <div className="app-body">
        <Sidebar />
        <Chat messages={messages} status={status} onSend={onSend} />
        <Board />
      </div>
    </div>
  )
}

export default AppShell
