import { useMemo } from 'react'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { classOf, type MessageFilter } from '../hooks/useMessageFilter'
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

const groupKey = (names: readonly string[]): string => [...names].sort().join('+')

const isAllMembersGroup = (names: readonly string[], combinableRoles: readonly string[]): boolean =>
  names.length === combinableRoles.length && names.every((n) => combinableRoles.includes(n))

const countSystemMessages = (messages: readonly ChatMessage[]): number => {
  let n = 0
  for (const m of messages) {
    if (classOf(m.messageType) === 'bpmux') n++
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
  mirrorRoster
}: Props): React.JSX.Element => {
  const systemCount = useMemo(() => countSystemMessages(messages), [messages])
  const roster = useTeamRoster({ mirrorRoster })
  const directRoles = useMemo(
    () => deriveDirectRolesFromRoster(roster, RENDERER_IDENTITY),
    [roster]
  )
  const combinableRoles = useMemo(
    () => deriveCombinableRolesFromRoster(roster, RENDERER_IDENTITY),
    [roster]
  )
  const groups = useMemo(() => deriveCombinatorialGroups(combinableRoles), [combinableRoles])
  const presence = usePresence(messages, directRoles)

  const bpmuxOnly = filter.visible.size === 1 && filter.visible.has('bpmux')
  const allMembersGroupSelected = !allView && recipient === null && !bpmuxOnly
  const bpmuxSelected = !allView && recipient === null && bpmuxOnly

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
  const goBpmux = (): void => {
    onSelectRecipient(null)
    filter.setOnly('bpmux')
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
                aria-label={`scope chat to ${names.join(' + ')}; owner ryan also observes`}
                data-test={`sidebar-group-${key}`}
                data-all-members={isAllMembers ? 'true' : undefined}
                onClick={() => goGroup(names)}
                title={`${names.join(' + ')} — owner ryan observes`}
              >
                <span className="sidebar-item-label">{names.join(' + ')}</span>
                <small
                  className="sidebar-item-owner-affordance"
                  data-test={`sidebar-group-${key}-owner`}
                  aria-hidden="true"
                >
                  +ryan
                </small>
              </button>
            )
          })
        )}
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">bpmux</h3>
        {systemCount === 0 ? (
          <p className="sidebar-empty" data-test="sidebar-bpmux-empty">
            none yet
          </p>
        ) : (
          <button
            className={`sidebar-item${bpmuxSelected ? ' sidebar-item--active' : ''}`}
            type="button"
            aria-current={bpmuxSelected || undefined}
            data-test="sidebar-bpmux"
            onClick={goBpmux}
          >
            bpmux ({systemCount})
          </button>
        )}
      </section>
    </nav>
  )
}

export default Sidebar
