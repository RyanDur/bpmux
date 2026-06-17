import { useEffect, useRef, useState } from 'react'
import Board from './board/Board'
import BoardResizeHandle from './board/BoardResizeHandle'
import Chat from './chat/Chat'
import CiGlyph from './ci/CiGlyph'
import { useBoardLayout } from './hooks/useBoardLayout'
import { useScope } from './hooks/useScope'
import { useTerminals } from './hooks/useTerminals'
import { useTheme } from './hooks/useTheme'
import { MoonIcon, SunIcon } from './icons/TitlebarIcons'
import Sidebar from './sidebar/Sidebar'
import type { ChatMessage, ConnectionStatus } from './transport/types'
import { type SendOptions } from './transport/useWebSocket'
import type { BoardResponse } from './board/types'
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
  boardSnapshot?: BoardResponse | null
  boardInvalidateTick: number
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
  boardSnapshot,
  boardInvalidateTick
}: Props): React.JSX.Element => {
  const {
    boardOpen,
    toggleBoardOpen,
    width: boardWidth,
    setBoardWidth,
    toggleBoardCollapse
  } = useBoardLayout()
  const confirmDialogRef = useRef<HTMLDialogElement>(null)
  const [metricsOpen, setMetricsOpen] = useState<boolean>(false)
  const { theme, toggle: toggleTheme } = useTheme()
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

  const { recipient, allView, filter, selectRecipient, selectAllView } =
    useScope(clearActiveTerminal)

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
        toggleBoardOpen()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [toggleBoardOpen])

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
          <CiGlyph />
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
        />
        <Chat
          messages={messages}
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
        />
        {boardOpen ? (
          <BoardResizeHandle
            width={boardWidth}
            setWidth={setBoardWidth}
            onToggleCollapse={toggleBoardCollapse}
            controlsId="board-region"
          />
        ) : null}
        <Board
          boardSnapshot={boardSnapshot}
          boardInvalidateTick={boardInvalidateTick}
          hidden={!boardOpen}
          width={boardWidth}
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
