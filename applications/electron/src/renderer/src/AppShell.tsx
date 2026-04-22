import { useEffect, useState } from 'react'
import Board from './board/Board'
import Chat from './chat/Chat'
import CiGlyph from './ci/CiGlyph'
import Sidebar from './sidebar/Sidebar'
import type { ChatMessage, ConnectionStatus } from './transport/types'
import type { SendOptions } from './transport/useWebSocket'
import './AppShell.css'

interface Props {
  status: ConnectionStatus
  messages: ChatMessage[]
  onSend: (text: string, opts?: SendOptions) => void
}

// Phase-1a window chrome: titlebar (app name, connection dot, CI
// glyph) on top; sidebar / chat / board regions below. Connection
// status surfaces as a small dot so a backend drop is visible without
// having to read the chat warning strip.
function AppShell({ status, messages, onSend }: Props): React.JSX.Element {
  const statusLabel =
    status === 'connected' ? 'connected' : status === 'connecting' ? 'connecting' : 'disconnected'

  const [boardOpen, setBoardOpen] = useState<boolean>(true)
  const [recipient, setRecipient] = useState<string | null>(null)

  useEffect(() => {
    const handler = (event: KeyboardEvent): void => {
      const ctrl = event.ctrlKey || event.metaKey
      if (!ctrl) return
      if (event.key !== 'p' && event.key !== 'P') return
      const target = event.target as HTMLElement | null
      const inEditable =
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLInputElement ||
        target?.isContentEditable === true
      if (inEditable) return
      event.preventDefault()
      setBoardOpen((prev) => !prev)
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [])

  return (
    <main className="app" data-test="app-root">
      <header className="titlebar" data-test="titlebar">
        <strong className="titlebar-app">bpmux</strong>
        <i className="titlebar-spacer" aria-hidden="true" />
        <i
          className={`titlebar-status titlebar-status--${status}`}
          title={statusLabel}
          aria-label={`connection ${statusLabel}`}
        />
        <CiGlyph />
      </header>
      <section className="app-body">
        <Sidebar recipient={recipient} onSelectRecipient={setRecipient} />
        <Chat
          messages={messages}
          status={status}
          recipient={recipient}
          onSend={onSend}
        />
        {boardOpen ? <Board /> : null}
      </section>
    </main>
  )
}

export default AppShell
