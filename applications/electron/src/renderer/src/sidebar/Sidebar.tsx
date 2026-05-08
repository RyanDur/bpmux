import { useMemo } from 'react'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { classOf, type MessageFilter } from '../hooks/useMessageFilter'
import { usePresence, type Presence } from '../hooks/usePresence'
import { useTeamRoster } from '../hooks/useTeamRoster'
import {
  deriveCombinableRoles,
  deriveCombinableRolesFromRoster,
  deriveCombinatorialGroups,
  deriveDirectRoles,
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
  apiBase?: string
  authToken?: string
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

const countSystemMessages = (messages: readonly ChatMessage[]): number => {
  let n = 0
  for (const m of messages) {
    if (classOf(m.messageType) === 'system') n++
  }
  return n
}

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
  apiBase,
  authToken,
  mirrorRoster
}: Props): React.JSX.Element => {
  const systemCount = useMemo(() => countSystemMessages(messages), [messages])
  const roster = useTeamRoster({ apiBase, authToken, messages, mirrorRoster })
  const peerRoster = useMemo(
    () => roster.filter((e) => e.role !== RENDERER_IDENTITY),
    [roster]
  )
  const directRoles = useMemo(() => {
    if (peerRoster.length > 0) {
      return deriveDirectRolesFromRoster(roster, RENDERER_IDENTITY)
    }
    return deriveDirectRoles(messages, RENDERER_IDENTITY)
  }, [peerRoster, roster, messages])
  const combinableRoles = useMemo(() => {
    if (peerRoster.length > 0) {
      return deriveCombinableRolesFromRoster(roster, RENDERER_IDENTITY)
    }
    return deriveCombinableRoles(messages, RENDERER_IDENTITY)
  }, [peerRoster, roster, messages])
  const groups = useMemo(() => deriveCombinatorialGroups(combinableRoles), [combinableRoles])
  const presence = usePresence(messages, directRoles)

  const systemOnly = filter.visible.size === 1 && filter.visible.has('system')
  const broadcastSelected = !allView && recipient === null && !systemOnly
  const systemSelected = !allView && recipient === null && systemOnly

  const goAll = (): void => {
    onSelectAllView()
    filter.reset()
  }
  const goBroadcast = (): void => {
    onSelectRecipient(null)
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
  const goSystem = (): void => {
    onSelectRecipient(null)
    filter.setOnly('system')
  }

  return (
    <nav className="sidebar" data-test="sidebar" aria-label="Channels">
      <section className="sidebar-section">
        <button
          className={`sidebar-item${allView ? ' sidebar-item--active' : ''}`}
          type="button"
          aria-current={allView || undefined}
          data-test="sidebar-all"
          onClick={goAll}
        >
          all conversations
        </button>
        <button
          className={`sidebar-item${broadcastSelected ? ' sidebar-item--active' : ''}`}
          type="button"
          aria-current={broadcastSelected || undefined}
          data-test="sidebar-broadcast"
          onClick={goBroadcast}
        >
          broadcast
        </button>
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">direct</h3>
        {directRoles.map((role) => {
          const active =
            !allView && recipient !== null && recipient.length === 1 && recipient[0] === role
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
              <span
                className={`presence-dot presence-dot--${status}`}
                data-test={`presence-dot-${role}`}
                data-presence={status}
                aria-label={`${role} ${statusLabel}`}
                title={`${role} — ${statusLabel}`}
              />
              <span className="sidebar-item-label">{role}</span>
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
        {groups.length === 0 ? (
          <p className="sidebar-empty" data-test="sidebar-groups-empty">
            none yet
          </p>
        ) : (
          groups.map((names) => {
            const key = groupKey(names)
            const active = !allView && recipientMatches(recipient, names)
            return (
              <button
                key={key}
                className={`sidebar-item${active ? ' sidebar-item--active' : ''}`}
                type="button"
                aria-current={active || undefined}
                data-test={`sidebar-group-${key}`}
                onClick={() => goGroup(names)}
                title={`scope chat to ${names.join(' + ')}`}
              >
                {names.join(' + ')}
              </button>
            )
          })
        )}
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">system</h3>
        {systemCount === 0 ? (
          <p className="sidebar-empty" data-test="sidebar-system-empty">
            none yet
          </p>
        ) : (
          <button
            className={`sidebar-item${systemSelected ? ' sidebar-item--active' : ''}`}
            type="button"
            aria-current={systemSelected || undefined}
            data-test="sidebar-system"
            onClick={goSystem}
          >
            system ({systemCount})
          </button>
        )}
      </section>
    </nav>
  )
}

export default Sidebar
