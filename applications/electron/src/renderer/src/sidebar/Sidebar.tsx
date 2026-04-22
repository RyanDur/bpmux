import './Sidebar.css'

interface Props {
  recipient: string | null
  onSelectRecipient: (r: string | null) => void
}

const DIRECT_ROLES = ['designer', 'dev', 'pm'] as const

function Sidebar({ recipient, onSelectRecipient }: Props): React.JSX.Element {
  const broadcastActive = recipient === null

  return (
    <nav className="sidebar" data-test="sidebar" aria-label="Channels">
      <section className="sidebar-section">
        <button
          className={`sidebar-item${broadcastActive ? ' sidebar-item--active' : ''}`}
          type="button"
          aria-current={broadcastActive || undefined}
          onClick={() => onSelectRecipient(null)}
        >
          broadcast
        </button>
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">direct</h3>
        {DIRECT_ROLES.map((role) => {
          const active = recipient === role
          return (
            <button
              key={role}
              className={`sidebar-item${active ? ' sidebar-item--active' : ''}`}
              type="button"
              aria-current={active || undefined}
              onClick={() => onSelectRecipient(role)}
            >
              {role}
            </button>
          )
        })}
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">groups</h3>
        <p className="sidebar-empty">none yet</p>
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">system</h3>
      </section>
    </nav>
  )
}

export default Sidebar
