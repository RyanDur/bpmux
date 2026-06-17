import { useEffect, useRef, useState } from 'react'
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
import { useTerminals } from './hooks/useTerminals'
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
import type { BoardResponse } from './board/types'
import { readScope, writeScope } from './lib/scopeStorage'
import './AppShell.css'

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
  boardSnapshot?: BoardResponse | null
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
  boardSnapshot,
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
  const confirmDialogRef = useRef<HTMLDialogElement>(null)
  const [metricsOpen, setMetricsOpen] = useState<boolean>(false)
  const { theme, toggle: toggleTheme } = useTheme()
  const channelKey = allView
    ? 'all'
    : recipient && recipient.length > 0
      ? `scoped:${[...recipient].sort().join(',')}`
      : 'broadcast'
  const filter = useMessageFilter(channelKey)
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined

  const {
    terminals,
    activeKey,
    terminalNames,
    confirmClose,
    spawnTerminal,
    restartTerminal,
    selectTerminal,
    renameTerminal,
    closeTerminal,
    confirmCloseTerminal,
    clearConfirmClose,
    clearActiveTerminal
  } = useTerminals(status, bridge)

  const selectRecipient = (r: string[] | null): void => {
    clearActiveTerminal()
    setAllView(false)
    setRecipient(r)
    writeScope(r && r.length > 0 ? { kind: 'scoped', value: r } : { kind: 'broadcast' })
  }

  const selectAllView = (): void => {
    clearActiveTerminal()
    setAllView(true)
    setRecipient(null)
    writeScope({ kind: 'all' })
  }

  useEffect(() => {
    if (confirmClose) confirmDialogRef.current?.showModal()
  }, [confirmClose])

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
          terminalNames={terminalNames}
          onRenameTerminal={renameTerminal}
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
          mirrorRoster={mirrorRoster}
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
          boardSnapshot={boardSnapshot}
          boardInvalidateTick={boardInvalidateTick}
          syncStatusTick={syncStatusTick}
          hidden={!boardOpen}
          width={boardWidthStorage.width}
        />
      </section>
      {confirmClose && (
        <dialog
          ref={confirmDialogRef}
          className="confirm-close"
          data-test="confirm-close-dialog"
          onCancel={clearConfirmClose}
        >
          <p className="confirm-close-message">
            A process is running in this terminal. Close it anyway?
          </p>
          <form method="dialog" className="confirm-close-actions">
            <button
              type="button"
              className="confirm-close-action"
              data-test="confirm-close-cancel"
              onClick={clearConfirmClose}
            >
              cancel
            </button>
            <button
              type="button"
              className="confirm-close-action confirm-close-confirm"
              data-test="confirm-close-confirm"
              onClick={confirmCloseTerminal}
            >
              close anyway
            </button>
          </form>
        </dialog>
      )}
    </main>
  )
}

export default AppShell
