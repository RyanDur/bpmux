import { useEffect, useMemo, useState } from 'react'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { type MessageFilter } from '../hooks/useMessageFilter'
import { PRESENCE, type Presence } from '../hooks/usePresence'
import { useTeamRoster } from '../hooks/useTeamRoster'
import {
  deriveCombinableRolesFromRoster,
  deriveCombinatorialGroups,
  deriveDirectRolesFromRoster
} from '../roles/deriveRoles'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import type { TerminalEntry } from '../AppShell'
import { resolveTerminalName } from '../lib/terminalNames'
import './Sidebar.css'

interface Props {
  recipient: string[] | null
  allView: boolean
  activeTerminalKey?: string | null
  terminals?: TerminalEntry[]
  onSelectRecipient: (r: string[] | null) => void
  onSelectAllView: () => void
  onAddTerminal?: () => void
  onSelectTerminal?: (key: string) => void
  onCloseTerminal?: (key: string) => void
  terminalNames?: Record<string, string>
  onRenameTerminal?: (id: string, name: string) => void
  messages: ChatMessage[]
  filter: MessageFilter
  mirrorRoster?: readonly RosterMemberWire[]
}

const PRESENCE_LABEL: Record<Presence, string> = {
  busy: 'busy',
  blocked: 'blocked',
  idle: 'idle',
  help: 'needs help',
  offline: 'offline',
  unknown: 'unknown'
}

const IO_FLICKER_WINDOW_MS = 2100
const EMPTY_ROLE_SET: ReadonlySet<string> = new Set()

const rolesInMessage = (msg: ChatMessage, peers: readonly string[]): ReadonlySet<string> => {
  const recipients = msg.to && msg.to.length > 0 ? msg.to : peers
  return new Set<string>([msg.from.split(':')[0], ...recipients])
}

const groupKey = (names: readonly string[]): string => [...names].sort().join('+')

const recipientMatches = (recipient: string[] | null, names: readonly string[]): boolean => {
  if (recipient === null) return false
  if (recipient.length !== names.length) return false
  return groupKey(recipient) === groupKey(names)
}

const Sidebar = ({
  recipient,
  allView,
  activeTerminalKey = null,
  terminals = [],
  onSelectRecipient,
  onSelectAllView,
  onAddTerminal = () => {},
  onSelectTerminal = () => {},
  onCloseTerminal = () => {},
  terminalNames = {},
  onRenameTerminal = () => {},
  messages,
  filter,
  mirrorRoster
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
  const roster = useTeamRoster({ mirrorRoster })
  const directRoles = useMemo(
    () => deriveDirectRolesFromRoster(roster, RENDERER_IDENTITY),
    [roster]
  )
  const combinableRoles = useMemo(
    () => deriveCombinableRolesFromRoster(roster, RENDERER_IDENTITY),
    [roster]
  )
  const partialGroups = useMemo(
    () =>
      deriveCombinatorialGroups(combinableRoles).filter(
        (names) => !(names.length === combinableRoles.length && combinableRoles.length >= 3)
      ),
    [combinableRoles]
  )
  const rosterPresenceByRole = useMemo(() => {
    const out = new Map<string, Presence>()
    for (const entry of roster) out.set(entry.role, entry.presence)
    return out
  }, [roster])

  const latestMessage = messages.length > 0 ? messages[messages.length - 1] : null
  const latestMessageId = latestMessage?.id ?? null
  const [baselineMessageId] = useState(() => latestMessageId)
  const [expiredMessageId, setExpiredMessageId] = useState<string | null>(null)
  useEffect(() => {
    if (latestMessageId === null || latestMessageId === baselineMessageId) return
    const timer = setTimeout(() => setExpiredMessageId(latestMessageId), IO_FLICKER_WINDOW_MS)
    return () => clearTimeout(timer)
  }, [latestMessageId, baselineMessageId])
  const ioActiveRoles = useMemo(() => {
    const live =
      latestMessage !== null &&
      latestMessageId !== baselineMessageId &&
      latestMessageId !== expiredMessageId
    return live ? rolesInMessage(latestMessage, directRoles) : EMPTY_ROLE_SET
  }, [latestMessage, latestMessageId, baselineMessageId, expiredMessageId, directRoles])

  const noTerminalActive = activeTerminalKey === null
  const allActive = allView && noTerminalActive

  const goAll = (): void => {
    onSelectAllView()
    filter.reset()
  }
  const goDirect = (role: string): void => {
    onSelectRecipient([role])
    filter.reset()
  }
  const goGroup = (names: string[]): void => {
    onSelectRecipient(names)
    filter.reset()
  }

  return (
    <nav className="sidebar" data-test="sidebar" aria-label="Channels">
      <section className="sidebar-section">
        <button
          className={`sidebar-item${allActive ? ' sidebar-item--active' : ''}`}
          type="button"
          aria-current={allActive || undefined}
          data-test="sidebar-all"
          title="meta-view of every channel"
          onClick={goAll}
        >
          all conversations
        </button>
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">direct</h3>
        {directRoles.map((role) => {
          const active =
            noTerminalActive &&
            !allView &&
            recipient !== null &&
            recipient.length === 1 &&
            recipient[0] === role
          const status = rosterPresenceByRole.get(role) ?? PRESENCE.Unknown
          const statusLabel = PRESENCE_LABEL[status]
          const ioActive = ioActiveRoles.has(role)
          return (
            <button
              key={role}
              className={`sidebar-item${active ? ' sidebar-item--active' : ''}`}
              type="button"
              aria-current={active || undefined}
              data-test={`sidebar-direct-${role}`}
              onClick={() => goDirect(role)}
            >
              <i
                className={`presence-dot presence-dot--${status}${ioActive ? ' presence-dot--io' : ''}`}
                data-test={`presence-dot-${role}`}
                data-presence={status}
                aria-label={`${role} ${statusLabel}`}
                title={`${role} — ${statusLabel}`}
              />
              {role}
              <small className="sidebar-item-status" data-test={`presence-label-${role}`}>
                {statusLabel}
              </small>
            </button>
          )
        })}
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">groups</h3>
        {partialGroups.length === 0 ? (
          <p className="sidebar-empty" data-test="sidebar-groups-empty">
            none yet
          </p>
        ) : (
          partialGroups.map((names) => {
            const key = groupKey(names)
            const active = noTerminalActive && !allView && recipientMatches(recipient, names)
            return (
              <button
                key={key}
                className={`sidebar-item sidebar-item--group${active ? ' sidebar-item--active' : ''}`}
                type="button"
                aria-current={active || undefined}
                aria-label={`scope chat to ${names.join(' + ')}`}
                data-test={`sidebar-group-${key}`}
                onClick={() => goGroup(names)}
                title={names.join(' + ')}
              >
                {names.join(' + ')}
              </button>
            )
          })
        )}
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading sidebar-heading--terminals">
          terminals
          <button
            className="sidebar-add-terminal"
            type="button"
            aria-label="spawn a new terminal"
            title="spawn a new terminal"
            data-test="sidebar-add-terminal"
            onClick={onAddTerminal}
          >
            ＋
          </button>
        </h3>
        {terminals.length === 0 ? (
          <p className="sidebar-empty" data-test="sidebar-terminals-empty">
            none yet
          </p>
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
                        data-test="sidebar-terminal-rename-input"
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
                      data-test="sidebar-terminal-item"
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
                    data-test="sidebar-terminal-close"
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
    </nav>
  )
}

export default Sidebar
