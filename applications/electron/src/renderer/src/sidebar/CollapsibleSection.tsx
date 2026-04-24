interface Props {
  id: string
  title: string
  collapsed: boolean
  onToggle: () => void
  children: React.ReactNode
}

const CollapsibleSection = ({
  id,
  title,
  collapsed,
  onToggle,
  children
}: Props): React.JSX.Element => {
  const contentId = `sidebar-section-${id}-content`
  return (
    <section className="sidebar-section">
      <button
        type="button"
        className="sidebar-heading sidebar-heading--toggle"
        aria-expanded={!collapsed}
        aria-controls={contentId}
        data-test={`sidebar-section-${id}-toggle`}
        onClick={onToggle}
      >
        <span className="sidebar-heading-chevron" aria-hidden="true" data-collapsed={collapsed}>
          {collapsed ? '▸' : '▾'}
        </span>
        <span>{title}</span>
      </button>
      {!collapsed && <div id={contentId}>{children}</div>}
    </section>
  )
}

export default CollapsibleSection
