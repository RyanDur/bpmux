import { useEffect, useState } from 'react'
import Board from './board/Board'
import Chat from './chat/Chat'
import CiGlyph from './ci/CiGlyph'
import { useDensity } from './hooks/useDensity'
import { useLaneSeparation } from './hooks/useLaneSeparation'
import { useMessageFilter } from './hooks/useMessageFilter'
import { useTheme } from './hooks/useTheme'
import {
  LanesIcon,
  MoonIcon,
  RowsComfortableIcon,
  RowsCompactIcon,
  StreamIcon,
  SunIcon
} from './icons/TitlebarIcons'
import Sidebar from './sidebar/Sidebar'
import type { ChatMessage, ConnectionStatus } from './transport/types'
import { RENDERER_IDENTITY, type SendOptions } from './transport/useWebSocket'
import './AppShell.css'

type PersistedScope =
  | { kind: 'all' }
  | { kind: 'broadcast' }
  | { kind: 'scoped'; value: string[] }

const SCOPE_STORAGE_KEY = `bpmux-scope:${RENDERER_IDENTITY}`

const readScope = (): PersistedScope | null => {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(SCOPE_STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') return null
    const obj = parsed as Record<string, unknown>
    if (obj.kind === 'all') return { kind: 'all' }
    if (obj.kind === 'broadcast') return { kind: 'broadcast' }
    if (obj.kind === 'scoped' && Array.isArray(obj.value)) {
      const value = obj.value.filter((v): v is string => typeof v === 'string')
      if (value.length > 0) return { kind: 'scoped', value }
    }
    return null
  } catch {
    return null
  }
}

const writeScope = (scope: PersistedScope): void => {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(SCOPE_STORAGE_KEY, JSON.stringify(scope))
  } catch {
    return
  }
}

interface Props {
  status: ConnectionStatus
  messages: ChatMessage[]
  onSend: (text: string, opts?: SendOptions) => void
}

const AppShell = ({ status, messages, onSend }: Props): React.JSX.Element => {
  const statusLabel =
    status === 'connected' ? 'connected' : status === 'connecting' ? 'connecting' : 'disconnected'

  const [boardOpen, setBoardOpen] = useState<boolean>(true)
  const initialScope = readScope()
  const [recipient, setRecipient] = useState<string[] | null>(
    initialScope?.kind === 'scoped' ? initialScope.value : null
  )
  const [allView, setAllView] = useState<boolean>(initialScope?.kind === 'all')
  const [metricsOpen, setMetricsOpen] = useState<boolean>(false)
  const { theme, toggle: toggleTheme } = useTheme()
  const { density, toggle: toggleDensity } = useDensity()
  const { lanes, toggle: toggleLanes } = useLaneSeparation()
  const filter = useMessageFilter()
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined

  const selectRecipient = (r: string[] | null): void => {
    setAllView(false)
    setRecipient(r)
    writeScope(r && r.length > 0 ? { kind: 'scoped', value: r } : { kind: 'broadcast' })
  }

  const selectAllView = (): void => {
    setAllView(true)
    setRecipient(null)
    writeScope({ kind: 'all' })
  }

  useEffect(() => {
    const handler = (event: KeyboardEvent): void => {
      const ctrl = event.ctrlKey || event.metaKey
      if (!ctrl) return
      const target = event.target as HTMLElement | null
      const inEditable =
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLInputElement ||
        target?.isContentEditable === true

      if ((event.key === 'm' || event.key === 'M') && !event.altKey && event.shiftKey) {
        event.preventDefault()
        setMetricsOpen((prev) => !prev)
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
        <span
          className={`titlebar-connection titlebar-connection--${status}`}
          title={`WebSocket connection: ${statusLabel}`}
          aria-label={`WebSocket connection ${statusLabel}`}
          data-test="titlebar-connection"
        >
          <i className={`titlebar-status titlebar-status--${status}`} aria-hidden="true" />
          <small className="titlebar-connection-label" data-test="titlebar-connection-label">
            {status === 'connected' ? 'online' : status === 'connecting' ? 'connecting…' : 'offline'}
          </small>
        </span>
        <i className="titlebar-divider" aria-hidden="true" />
        <button
          type="button"
          className="titlebar-theme-toggle"
          onClick={toggleTheme}
          aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
          title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        >
          {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
        </button>
        <button
          type="button"
          className="titlebar-density-toggle"
          onClick={toggleDensity}
          aria-pressed={density === 'compact'}
          aria-label={
            density === 'compact' ? 'Switch to comfortable density' : 'Switch to compact density'
          }
          title={
            density === 'compact' ? 'Switch to comfortable density' : 'Switch to compact density'
          }
          data-test="titlebar-density-toggle"
          data-density={density}
        >
          {density === 'compact' ? <RowsCompactIcon /> : <RowsComfortableIcon />}
        </button>
        <button
          type="button"
          className="titlebar-lanes-toggle"
          onClick={toggleLanes}
          aria-pressed={lanes === 'on'}
          aria-label={
            lanes === 'on' ? 'Switch to interleaved messages' : 'Switch to lane-separated messages'
          }
          title={
            lanes === 'on' ? 'Switch to interleaved messages' : 'Switch to lane-separated messages'
          }
          data-test="titlebar-lanes-toggle"
          data-lanes={lanes}
        >
          {lanes === 'on' ? <LanesIcon /> : <StreamIcon />}
        </button>
        <CiGlyph messages={messages} />
      </header>
      <section className="app-body">
        <Sidebar
          recipient={recipient}
          allView={allView}
          onSelectRecipient={selectRecipient}
          onSelectAllView={selectAllView}
          messages={messages}
          filter={filter}
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
        />
        <Chat
          messages={messages}
          status={status}
          recipient={recipient}
          allView={allView}
          onSelectRecipient={selectRecipient}
          filter={filter}
          metricsOpen={metricsOpen}
          onCloseMetrics={() => setMetricsOpen(false)}
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
          lanes={lanes}
          onSend={onSend}
        />
        <Board messages={messages} hidden={!boardOpen} />
      </section>
    </main>
  )
}

export default AppShell
