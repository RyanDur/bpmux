import './Sidebar.css'

// Phase-1a sidebar: static channel scaffold. Broadcast is selected;
// direct / groups / system are placeholders (no routing yet — all
// messages stream into the one chat region). Selectable channels are
// phase-1b; this gives the layout its left column without pretending
// to route.
function Sidebar(): React.JSX.Element {
  return (
    <nav className="sidebar" data-test="sidebar" aria-label="Channels">
      <section className="sidebar-section">
        <button className="sidebar-item sidebar-item--active" type="button" aria-current="true">
          broadcast
        </button>
      </section>
      <section className="sidebar-section">
        <h3 className="sidebar-heading">direct</h3>
        <button className="sidebar-item sidebar-item--muted" type="button" disabled>
          designer
        </button>
        <button className="sidebar-item sidebar-item--muted" type="button" disabled>
          dev
        </button>
        <button className="sidebar-item sidebar-item--muted" type="button" disabled>
          pm
        </button>
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
