import './Sidebar.css'

// Phase-1a sidebar: static channel scaffold. Broadcast is selected;
// direct / groups / system are placeholders (no routing yet — all
// messages stream into the one chat region). Selectable channels are
// phase-1b; this gives the layout its left column without pretending
// to route.
function Sidebar(): React.JSX.Element {
  return (
    <aside className="sidebar" data-test="sidebar">
      <div className="sidebar-section">
        <button className="sidebar-item sidebar-item--active" type="button">
          broadcast
        </button>
      </div>
      <div className="sidebar-section">
        <div className="sidebar-heading">direct</div>
        <button className="sidebar-item sidebar-item--muted" type="button" disabled>
          designer
        </button>
        <button className="sidebar-item sidebar-item--muted" type="button" disabled>
          dev
        </button>
        <button className="sidebar-item sidebar-item--muted" type="button" disabled>
          pm
        </button>
      </div>
      <div className="sidebar-section">
        <div className="sidebar-heading">groups</div>
        <div className="sidebar-empty">none yet</div>
      </div>
      <div className="sidebar-section">
        <div className="sidebar-heading">system</div>
      </div>
    </aside>
  )
}

export default Sidebar
