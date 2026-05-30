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
import './Sidebar.css'

interface Props {
  recipient: string[] | null
  allView: boolean
  onSelectRecipient: (r: string[] | null) => void
  onSelectAllView: () => void
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

const DOT_LABEL: Record<string, string> = {
  dead: 'terminal exited',
  'needs-input': 'needs input'
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
  onSelectRecipient,
  onSelectAllView,
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

  const allMembersGroupSelected = !allView && recipient === null

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
          className={`sidebar-item${allView ? ' sidebar-item--active' : ''}`}
          type="button"
          aria-current={allView || undefined}
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
            !allView && recipient !== null && recipient.length === 1 && recipient[0] === role
          const status = presence.get(role) ?? 'offline'
          const statusLabel = PRESENCE_LABEL[status]
          const terminalState = mirrorRoster?.find(
            (m) => m.name === role || m.identity.split(':')[0] === role
          )?.terminal_state
          const needsInput = status === 'help' || status === 'blocked'
          const dotState = terminalState === 'dead' ? 'dead' : needsInput ? 'needs-input' : status
          const dotLabel = DOT_LABEL[dotState] ?? statusLabel
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
                className={`presence-dot presence-dot--${dotState}`}
                data-test={`presence-dot-${role}`}
                data-presence={status}
                data-terminal-state={terminalState ?? undefined}
                aria-label={`${role} ${dotLabel}`}
                title={`${role} — ${dotLabel}`}
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
              : !allView && recipientMatches(recipient, names)
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
    </nav>
  )
}

export default Sidebar
