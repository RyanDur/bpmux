import { useMemo } from 'react'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { type MessageFilter } from '../hooks/useMessageFilter'
import { usePresence, type Presence } from '../hooks/usePresence'
import { useTeamRoster } from '../hooks/useTeamRoster'
import {
  deriveCombinableRolesFromRoster,
  deriveCombinatorialGroups,
  deriveDirectRolesFromRoster
} from '../roles/deriveRoles'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import type { TerminalEntry } from '../AppShell'
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
  messages: ChatMessage[]
  filter: MessageFilter
  mirrorRoster?: readonly RosterMemberWire[]
}

const PRESENCE_LABEL: Record<Presence, string> = {
  available: 'available',
  busy: 'busy',
  blocked: 'blocked',
  idle: 'idle',
  help: 'needs help',
  offline: 'offline'
}

const groupKey = (names: readonly string[]): string => [...names].sort().join('+')

const isAllMembersGroup = (names: readonly string[], combinableRoles: readonly string[]): boolean =>
  names.length === combinableRoles.length && names.every((n) => combinableRoles.includes(n))

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
  messages,
  filter,
  mirrorRoster
}: Props): React.JSX.Element => {
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
  const presence = usePresence(messages, directRoles)

  const noTerminalActive = activeTerminalKey === null
  const allMembersGroupSelected = !allView && recipient === null && noTerminalActive
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
    if (isAllMembersGroup(names, combinableRoles)) {
      onSelectRecipient(null)
    } else {
      onSelectRecipient(names)
    }
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
          const status = presence.get(role) ?? 'offline'
          const statusLabel = PRESENCE_LABEL[status]
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
                className={`presence-dot presence-dot--${status}`}
                data-test={`presence-dot-${role}`}
                data-presence={status}
                aria-label={`${role} ${statusLabel}`}
                title={`${role} — ${statusLabel}`}
              />
              {role}
              {status !== 'offline' && status !== 'available' && (
                <small className="sidebar-item-status" data-test={`presence-label-${role}`}>
                  {statusLabel}
                </small>
              )}
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
            const isAllMembers = isAllMembersGroup(names, combinableRoles)
            const active = isAllMembers
              ? allMembersGroupSelected
              : noTerminalActive && !allView && recipientMatches(recipient, names)
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
              const label =
                t.state === 'ready'
                  ? `${i + 1} · ${dir}`
                  : t.state === 'spawning'
                    ? `${i + 1} · spawning…`
                    : t.state === 'spawn-failed'
                      ? `${i + 1} · failed`
                      : `${i + 1} · exited`
              return (
                <li key={t.key} className="sidebar-terminal-row">
                  <button
                    className={`sidebar-item${active ? ' sidebar-item--active' : ''}`}
                    type="button"
                    aria-current={active || undefined}
                    data-test="sidebar-terminal-item"
                    data-terminal-key={t.key}
                    data-state={t.state}
                    title={t.cwd ?? t.state}
                    onClick={() => onSelectTerminal(t.key)}
                  >
                    {label}
                  </button>
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
