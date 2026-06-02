import { useEffect, useState } from 'react'
import Board from './board/Board'
import BoardResizeHandle from './board/BoardResizeHandle'
import {
  BOARD_DEFAULT_WIDTH,
  clampBoardWidth,
  computeCollapseToggle
} from './board/BoardResizeHandle.utils'
import Chat from './chat/Chat'
import CiGlyph from './ci/CiGlyph'
import { useMessageFilter } from './hooks/useMessageFilter'
import { useTheme } from './hooks/useTheme'
import { MoonIcon, SunIcon } from './icons/TitlebarIcons'
import { useBoardWidthStorage } from './lib/useBoardWidthStorage'
import Sidebar from './sidebar/Sidebar'
import type {
  ChatMessage,
  CiSnapshotWire,
  ConnectionStatus,
  ReactionAggregateWire,
  RosterMemberWire
} from './transport/types'
import { RENDERER_IDENTITY, type SendOptions } from './transport/useWebSocket'
import { subscribeWsPtyExit } from './transport/wsService'
import './AppShell.css'

type PersistedScope = { kind: 'all' } | { kind: 'broadcast' } | { kind: 'scoped'; value: string[] }

export type TerminalState = 'spawning' | 'ready' | 'spawn-failed' | 'exited'

export interface TerminalEntry {
  key: string
  state: TerminalState
  id?: string
  cwd?: string
  error?: string
}

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
  loadOlder?: () => Promise<void>
  isLoadingOlder?: boolean
  hasMoreOlder?: boolean
  hasError?: boolean
  onAtBottomChange?: (atBottom: boolean) => void
  ciSnapshot?: CiSnapshotWire | null
  mirrorReactions?: ReadonlyMap<string, ReactionAggregateWire[]>
  mirrorRoster?: readonly RosterMemberWire[]
  boardInvalidateTick: number
  syncStatusTick: number
}

const AppShell = ({
  status,
  messages,
  onSend,
  loadOlder,
  isLoadingOlder,
  hasMoreOlder,
  hasError,
  onAtBottomChange,
  ciSnapshot,
  mirrorReactions,
  mirrorRoster,
  boardInvalidateTick,
  syncStatusTick
}: Props): React.JSX.Element => {
  const [boardOpen, setBoardOpen] = useState<boolean>(true)
  const boardWidthStorage = useBoardWidthStorage(
    'bpmux-board-width',
    RENDERER_IDENTITY,
    BOARD_DEFAULT_WIDTH
  )
  const [restoreWidth, setRestoreWidth] = useState<number>(boardWidthStorage.width)
  const setBoardWidth = (next: number): void => {
    if (typeof window === 'undefined') {
      boardWidthStorage.setWidth(next)
      return
    }
    boardWidthStorage.setWidth(clampBoardWidth(next, window.innerWidth))
  }
  const toggleBoardCollapse = (viewportWidth: number): void => {
    const { nextWidth, nextRestoreWidth } = computeCollapseToggle(
      boardWidthStorage.width,
      restoreWidth,
      viewportWidth
    )
    setRestoreWidth(nextRestoreWidth)
    boardWidthStorage.setWidth(nextWidth)
  }
  const initialScope = readScope()
  const [recipient, setRecipient] = useState<string[] | null>(
    initialScope?.kind === 'scoped' ? initialScope.value : null
  )
  const [allView, setAllView] = useState<boolean>(initialScope?.kind !== 'scoped')
  const [terminals, setTerminals] = useState<TerminalEntry[]>([])
  const [activeKey, setActiveKey] = useState<string | null>(null)
  const [metricsOpen, setMetricsOpen] = useState<boolean>(false)
  const { theme, toggle: toggleTheme } = useTheme()
  const channelKey = allView
    ? 'all'
    : recipient && recipient.length > 0
      ? `scoped:${[...recipient].sort().join(',')}`
      : 'broadcast'
  const filter = useMessageFilter(channelKey)
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined

  const selectRecipient = (r: string[] | null): void => {
    setActiveKey(null)
    setAllView(false)
    setRecipient(r)
    writeScope(r && r.length > 0 ? { kind: 'scoped', value: r } : { kind: 'broadcast' })
  }

  const selectAllView = (): void => {
    setActiveKey(null)
    setAllView(true)
    setRecipient(null)
    writeScope({ kind: 'all' })
  }

  const requestSpawn = (key: string): void => {
    if (!bridge) return
    const markFailed = (error: string): void =>
      setTerminals((prev) =>
        prev.map((t) => (t.key === key ? { ...t, state: 'spawn-failed', error } : t))
      )
    void fetch(`${bridge.apiBase}/api/terminals/spawn`, {
      method: 'POST',
      headers: { authorization: `Bearer ${bridge.authToken}` }
    })
      .then(async (res) => {
        if (!res.ok) {
          const detail = await res.text().catch(() => '')
          markFailed(detail.length > 0 ? detail : `spawn failed (${res.status})`)
          return
        }
        const body = (await res.json()) as { terminal_id?: string; cwd?: string }
        if (!body.terminal_id) {
          markFailed('spawn response missing terminal_id')
          return
        }
        const id = body.terminal_id
        setTerminals((prev) =>
          prev.map((t) => (t.key === key ? { ...t, state: 'ready', id, cwd: body.cwd ?? '' } : t))
        )
      })
      .catch((e: unknown) => markFailed(String(e)))
  }

  const spawnTerminal = (): void => {
    if (!bridge) return
    const key = crypto.randomUUID()
    setTerminals((prev) => [...prev, { key, state: 'spawning' }])
    setActiveKey(key)
    requestSpawn(key)
  }

  const restartTerminal = (key: string): void => {
    if (!bridge) return
    setTerminals((prev) => prev.map((t) => (t.key === key ? { key, state: 'spawning' } : t)))
    setActiveKey(key)
    requestSpawn(key)
  }

  const selectTerminal = (key: string): void => {
    setActiveKey(key)
  }

  const closeTerminal = (key: string): void => {
    const closing = terminals.find((t) => t.key === key)
    if (closing?.id && bridge) {
      void fetch(`${bridge.apiBase}/api/terminals/close`, {
        method: 'POST',
        headers: {
          authorization: `Bearer ${bridge.authToken}`,
          'content-type': 'application/json'
        },
        body: JSON.stringify({ terminal_id: closing.id })
      }).catch((e: unknown) => console.error('bpmux: terminal close failed', e))
    }
    const remaining = terminals.filter((t) => t.key !== key)
    setTerminals(remaining)
    if (activeKey === key) {
      setActiveKey(remaining.length > 0 ? remaining[remaining.length - 1].key : null)
    }
  }

  useEffect(() => {
    return subscribeWsPtyExit((terminalId) => {
      setTerminals((prev) => prev.map((t) => (t.id === terminalId ? { ...t, state: 'exited' } : t)))
    })
  }, [])

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
        <strong
          className={`titlebar-app titlebar-app--${status}`}
          data-status={status}
          data-test="titlebar-app"
          aria-label={`bpmux ${status === 'connected' ? 'online' : status === 'connecting' ? 'reconnecting' : 'offline'}`}
        >
          bpmux
          <small className="titlebar-app-state" data-test="titlebar-app-state">
            {status === 'connected'
              ? 'online'
              : status === 'connecting'
                ? 'reconnecting'
                : 'offline'}
          </small>
        </strong>
        <button
          type="button"
          className="titlebar-theme-toggle"
          onClick={toggleTheme}
          aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
          title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        >
          {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
        </button>
        <output data-test="titlebar-ci">
          <CiGlyph ciSnapshot={ciSnapshot} />
        </output>
      </header>
      <section className="app-body">
        <Sidebar
          recipient={recipient}
          allView={allView}
          activeTerminalKey={activeKey}
          terminals={terminals}
          onSelectRecipient={selectRecipient}
          onSelectAllView={selectAllView}
          onAddTerminal={spawnTerminal}
          onSelectTerminal={selectTerminal}
          onCloseTerminal={closeTerminal}
          messages={messages}
          filter={filter}
          mirrorRoster={mirrorRoster}
        />
        <Chat
          messages={messages}
          status={status}
          recipient={recipient}
          allView={allView}
          activeKey={activeKey}
          terminals={terminals}
          onSelectRecipient={selectRecipient}
          onCloseTerminal={closeTerminal}
          onRestartTerminal={restartTerminal}
          filter={filter}
          metricsOpen={metricsOpen}
          onCloseMetrics={() => setMetricsOpen(false)}
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
          onSend={onSend}
          loadOlder={loadOlder}
          isLoadingOlder={isLoadingOlder}
          hasMoreOlder={hasMoreOlder}
          hasError={hasError}
          onAtBottomChange={onAtBottomChange}
          mirrorReactions={mirrorReactions}
        />
        {boardOpen ? (
          <BoardResizeHandle
            width={boardWidthStorage.width}
            setWidth={setBoardWidth}
            onToggleCollapse={toggleBoardCollapse}
            controlsId="board-region"
          />
        ) : null}
        <Board
          boardInvalidateTick={boardInvalidateTick}
          syncStatusTick={syncStatusTick}
          hidden={!boardOpen}
          width={boardWidthStorage.width}
        />
      </section>
    </main>
  )
}

export default AppShell
