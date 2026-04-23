import { useMemo } from 'react'
import type { ChatMessage } from '../transport/types'
import { classOf, type MessageFilter } from '../hooks/useMessageFilter'
import { usePresence, type Presence } from '../hooks/usePresence'
import {
  deriveCombinableRoles,
  deriveCombinatorialGroups,
  deriveDirectRoles
} from '../roles/deriveRoles'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import './Sidebar.css'

interface Props {
  recipient: string[] | null
  onSelectRecipient: (r: string[] | null) => void
  messages: ChatMessage[]
  filter: MessageFilter
}

// Human label for each presence state. Mapping + ordering kept here
// (not in the hook) so the UI can iterate without committing the
// derivation module to rendering concerns.
const PRESENCE_LABEL: Record<Presence, string> = {
  available: 'available',
  busy: 'busy',
  blocked: 'blocked',
  idle: 'idle',
  help: 'needs help',
  offline: 'offline'
}

// Stable string key for a group's recipient set. Sorted so the
// same group (regardless of order the message was addressed) renders
// as one sidebar item. Joined with '+' — readable both in the label
// ('dev+designer') and in React keys.
function groupKey(names: readonly string[]): string {
  return [...names].sort().join('+')
}

// #386 (supersedes #346): groups are combinatorial over the observed
// team, NOT derived from message history. History-derived groups
// create phantom-group ghosts from stale message-routing; Ryan
// 2026-04-22 22:55 UTC: "groups are a need. but if we use history,
// we will create ghosts." All 2+ subsets of the team (excluding the
// all-team subset which is redundant with broadcast) render as
// first-class groups the owner can pick + scope to. Derivation is
// pure function of deriveTeamRoles output, so a 5th role joining
// via message stream automatically expands the group list.

function countSystemMessages(messages: readonly ChatMessage[]): number {
  let n = 0
  for (const m of messages) {
    if (classOf(m.messageType) === 'system') n++
  }
  return n
}

function recipientMatches(recipient: string[] | null, names: readonly string[]): boolean {
  if (recipient === null) return false
  if (recipient.length !== names.length) return false
  return groupKey(recipient) === groupKey(names)
}

function Sidebar({
  recipient,
  onSelectRecipient,
  messages,
  filter
}: Props): React.JSX.Element {
  const systemCount = useMemo(() => countSystemMessages(messages), [messages])
  // #393: direct-DM roles are every role present in the session
  // except the viewer's own. Any agent that joins becomes a
  // first-class sidebar entry without a code change.
  const directRoles = useMemo(
    () => deriveDirectRoles(messages, RENDERER_IDENTITY),
    [messages]
  )
  // #386 + Ryan 2026-04-23 11:47 UTC: combinatorial groups enumerate
  // 2+ subsets over the "combinable" role set — observed team minus
  // the owner (viewer) and system senders. Ryan: "my name should not
  // be part of the list of combinations, nor should bpmux. we are
  // constants." Detection is structural (viewer via RENDERER_IDENTITY,
  // system senders via messageType==='system') so a fork with
  // different owner/system names works without code edit — no literal
  // role-name strings in the helper (preserves #393 AC1).
  const combinableRoles = useMemo(
    () => deriveCombinableRoles(messages, RENDERER_IDENTITY),
    [messages]
  )
  const groups = useMemo(
    () => deriveCombinatorialGroups(combinableRoles),
    [combinableRoles]
  )
  const presence = usePresence(messages, directRoles)
  // Broadcast is "selected" whenever recipient is null. SYSTEM is
  // "selected" specifically when the filter is narrowed to system-only
  // (the state goSystem puts us in). Both share recipient===null but
  // differ on filter state — the sidebar shows exactly one item active
  // at a time so the navigation metaphor holds.
  const broadcastSelected =
    recipient === null && !(filter.visible.size === 1 && filter.visible.has('system'))
  const systemSelected =
    filter.visible.size === 1 && filter.visible.has('system') && recipient === null

  // Navigation helpers wire sidebar clicks into BOTH recipient and
  // filter. Selecting a DM or group resets the filter to default
  // (chat+insight) so the user sees a normal chat scope. Clicking
  // SYSTEM narrows the filter to system-only (AC4). Broadcast restores
  // the default filter so the user returns to the common view.
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
          const active = recipient !== null && recipient.length === 1 && recipient[0] === role
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
            const active = recipientMatches(recipient, names)
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
