import { useId } from 'react'
import './Tabs.css'

export interface TabDef<TabId extends string> {
  id: TabId
  label: string
  tabTestId?: string
  panelTestId?: string
}

export interface TabsProps<TabId extends string> {
  ariaLabel: string
  tabs: ReadonlyArray<TabDef<TabId>>
  activeId: TabId
  onActiveChange: (id: TabId) => void
  renderPanel: (id: TabId) => React.ReactNode
  className?: string
  panelClassName?: string
}

export const Tabs = <TabId extends string>({
  ariaLabel,
  tabs,
  activeId,
  onActiveChange,
  renderPanel,
  className,
  panelClassName
}: TabsProps<TabId>): React.JSX.Element => {
  const groupName = useId()
  return (
    <section className={`tabs${className ? ` ${className}` : ''}`}>
      <fieldset className="tabs-fieldset">
        <legend className="tabs-legend">{ariaLabel}</legend>
        {tabs.map((tab) => (
          <label key={tab.id} className="tabs-label" data-test={tab.tabTestId}>
            <input
              type="radio"
              name={groupName}
              value={tab.id}
              checked={tab.id === activeId}
              onChange={() => onActiveChange(tab.id)}
              className="tabs-radio"
            />
            {tab.label}
          </label>
        ))}
      </fieldset>
      {tabs.map((tab) => (
        <article
          key={tab.id}
          aria-label={tab.label}
          className={`tabs-panel${panelClassName ? ` ${panelClassName}` : ''}`}
          data-test={tab.panelTestId}
          hidden={tab.id !== activeId}
        >
          {renderPanel(tab.id)}
        </article>
      ))}
    </section>
  )
}

export default Tabs
