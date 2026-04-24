import type { ReactNode } from 'react'
import './CollapsibleSection.css'

interface Props {
  testId: string
  title: ReactNode
  collapsed: boolean
  onToggle: () => void
  children: ReactNode
  className?: string
  toggleClassName?: string
}

const CollapsibleSection = ({
  testId,
  title,
  collapsed,
  onToggle,
  children,
  className,
  toggleClassName
}: Props): React.JSX.Element => {
  const contentId = `${testId}-content`
  const outerClass = className ?? 'collapsible-section'
  const buttonClass = toggleClassName ?? 'collapsible-section-toggle'
  return (
    <section className={outerClass}>
      <button
        type="button"
        className={buttonClass}
        aria-expanded={!collapsed}
        aria-controls={contentId}
        data-test={testId}
        onClick={onToggle}
      >
        <span className="collapsible-section-chevron" aria-hidden="true" data-collapsed={collapsed}>
          {collapsed ? '▸' : '▾'}
        </span>
        <span className="collapsible-section-title">{title}</span>
      </button>
      {!collapsed && <div id={contentId}>{children}</div>}
    </section>
  )
}

export default CollapsibleSection
