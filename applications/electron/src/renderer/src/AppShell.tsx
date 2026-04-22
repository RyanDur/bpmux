import { useEffect, useState } from 'react'
import Board from './board/Board'
import Chat from './chat/Chat'
import CiGlyph from './ci/CiGlyph'
import { useTheme } from './hooks/useTheme'
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
  const [searchOpen, setSearchOpen] = useState<boolean>(false)
  const { theme, toggle: toggleTheme } = useTheme()
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined

  useEffect(() => {
    const handler = (event: KeyboardEvent): void => {
      const ctrl = event.ctrlKey || event.metaKey
      if (!ctrl) return
      const target = event.target as HTMLElement | null
      const inEditable =
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLInputElement ||
        target?.isContentEditable === true

      if ((event.key === 'f' || event.key === 'F') && !event.altKey && !event.shiftKey) {
        event.preventDefault()
        setSearchOpen(true)
        return
      }

      if (event.key === 'p' || event.key === 'P') {
        if (inEditable) return
        event.preventDefault()
        setBoardOpen((prev) => !prev)
      }
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
        <button
          type="button"
          className="titlebar-theme-toggle"
          onClick={toggleTheme}
          aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
          title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        >
          {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
        </button>
        <CiGlyph />
      </header>
      <section className="app-body">
        <Sidebar recipient={recipient} onSelectRecipient={setRecipient} />
        <Chat
          messages={messages}
          status={status}
          recipient={recipient}
          searchOpen={searchOpen}
          onCloseSearch={() => setSearchOpen(false)}
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
          onSend={onSend}
        />
        {boardOpen ? <Board /> : null}
      </section>
    </main>
  )
}

function SunIcon(): React.JSX.Element {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" focusable="false">
      <circle cx="7" cy="7" r="3" fill="currentColor" />
      <g stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
        <line x1="7" y1="1" x2="7" y2="2.5" />
        <line x1="7" y1="11.5" x2="7" y2="13" />
        <line x1="1" y1="7" x2="2.5" y2="7" />
        <line x1="11.5" y1="7" x2="13" y2="7" />
        <line x1="2.5" y1="2.5" x2="3.5" y2="3.5" />
        <line x1="10.5" y1="10.5" x2="11.5" y2="11.5" />
        <line x1="2.5" y1="11.5" x2="3.5" y2="10.5" />
        <line x1="10.5" y1="3.5" x2="11.5" y2="2.5" />
      </g>
    </svg>
  )
}

function MoonIcon(): React.JSX.Element {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" focusable="false">
      <path
        d="M11.5 8.5a4.5 4.5 0 0 1-6-6 5 5 0 1 0 6 6z"
        fill="currentColor"
      />
    </svg>
  )
}

export default AppShell
