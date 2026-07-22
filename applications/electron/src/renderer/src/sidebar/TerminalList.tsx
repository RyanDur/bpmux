import { useState } from 'react'
import type { TerminalEntry } from '../terminal/useTerminals'
import { resolveTerminalName } from '../terminal/terminalNames'

interface Props {
  terminals: TerminalEntry[]
  activeTerminalKey: string | null
  terminalNames: Record<string, string>
  onAddTerminal: () => void
  onSelectTerminal: (key: string) => void
  onCloseTerminal: (key: string) => void
  onRenameTerminal: (id: string, name: string) => void
}

const TerminalList = ({
  terminals,
  activeTerminalKey,
  terminalNames,
  onAddTerminal,
  onSelectTerminal,
  onCloseTerminal,
  onRenameTerminal
}: Props): React.JSX.Element => {
  const [editingKey, setEditingKey] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const [returnFocusKey, setReturnFocusKey] = useState<string | null>(null)
  const beginRename = (key: string, currentName: string): void => {
    setDraft(currentName)
    setEditingKey(key)
  }
  const closeEdit = (returnFocus: boolean): void => {
    if (returnFocus) setReturnFocusKey(editingKey)
    setEditingKey(null)
  }
  const commitRename = (id: string, returnFocus: boolean): void => {
    onRenameTerminal(id, draft)
    closeEdit(returnFocus)
  }

  return (
    <section className="sidebar-section">
      <h3 className="sidebar-heading sidebar-heading--terminals">
        terminals
        <button
          className="sidebar-add-terminal"
          type="button"
          aria-label="spawn a new terminal"
          title="spawn a new terminal"
          onClick={onAddTerminal}
        >
          ＋
        </button>
      </h3>
      {terminals.length === 0 ? (
        <p className="sidebar-empty">none yet</p>
      ) : (
        <ul className="sidebar-terminal-list">
          {terminals.map((t, i) => {
            const active = activeTerminalKey === t.key
            const dir = (t.cwd ?? '').split('/').filter(Boolean).pop() ?? '/'
            const id = t.id
            const custom = id !== undefined ? terminalNames[id] : undefined
            const renamable = t.state === 'ready' && id !== undefined
            const label =
              t.state === 'ready'
                ? `${i + 1} · ${resolveTerminalName(custom, dir)}`
                : t.state === 'spawning'
                  ? `${i + 1} · spawning…`
                  : t.state === 'spawn-failed'
                    ? `${i + 1} · failed`
                    : `${i + 1} · exited`
            return (
              <li key={t.key} className="sidebar-terminal-row">
                {editingKey === t.key && id !== undefined ? (
                  <label className="sidebar-item sidebar-terminal-rename">
                    {`${i + 1} · `}
                    <input
                      className="sidebar-terminal-rename-input"
                      data-terminal-key={t.key}
                      aria-label={`rename terminal ${i + 1}`}
                      value={draft}
                      autoFocus
                      onChange={(e) => setDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          e.preventDefault()
                          commitRename(id, true)
                        } else if (e.key === 'Escape') {
                          e.preventDefault()
                          closeEdit(true)
                        }
                      }}
                      onBlur={() => commitRename(id, false)}
                    />
                  </label>
                ) : (
                  <button
                    className={`sidebar-item${active ? ' sidebar-item--active' : ''}`}
                    type="button"
                    autoFocus={returnFocusKey === t.key}
                    aria-current={active || undefined}
                    data-terminal-key={t.key}
                    data-state={t.state}
                    title={t.cwd ?? t.state}
                    onFocus={() => {
                      if (returnFocusKey === t.key) setReturnFocusKey(null)
                    }}
                    onClick={() => onSelectTerminal(t.key)}
                    onDoubleClick={() => {
                      if (renamable) beginRename(t.key, custom ?? '')
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'F2' && renamable) {
                        e.preventDefault()
                        beginRename(t.key, custom ?? '')
                      }
                    }}
                  >
                    {label}
                  </button>
                )}
                <button
                  className="sidebar-terminal-close"
                  type="button"
                  aria-label={`close terminal ${i + 1}`}
                  title="close terminal"
                  data-terminal-key={t.key}
                  onClick={() => onCloseTerminal(t.key)}
                >
                  ✕
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}

export default TerminalList
