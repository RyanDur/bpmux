import type { ConnectionStatus } from '../types'

interface Props {
  status: ConnectionStatus
}

// Phase-1a titlebar: app name + placeholder CI glyph (real CI hook is
// a phase-1a follow-up per the spec — text placeholder is fine). Also
// surfaces connection status as a small dot so the user notices when
// the backend drops without having to read the chat warning strip.
function Titlebar({ status }: Props): React.JSX.Element {
  const statusLabel =
    status === 'connected' ? 'connected' : status === 'connecting' ? 'connecting' : 'disconnected'

  return (
    <header className="titlebar" data-test="titlebar">
      <span className="titlebar-app">bpmux</span>
      <span className="titlebar-spacer" />
      <span
        className={`titlebar-status titlebar-status--${status}`}
        title={statusLabel}
        aria-label={`connection ${statusLabel}`}
      />
      <span className="titlebar-ci" title="CI status (coming soon)">
        CI ?
      </span>
    </header>
  )
}

export default Titlebar
