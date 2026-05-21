import { useCallback, useRef } from 'react'
import './Tabs.css'

export interface TabDef<TabId extends string> {
  id: TabId
  label: string
  panelTestId?: string
  tabTestId?: string
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

const focusTabAt = (refs: React.RefObject<Map<string, HTMLButtonElement>>, id: string): void => {
  const el = refs.current?.get(id)
  if (el) el.focus()
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
  const tabRefs = useRef<Map<string, HTMLButtonElement>>(new Map())
  const setTabRef = useCallback(
    (id: TabId) =>
      (el: HTMLButtonElement | null): void => {
        if (el) tabRefs.current.set(id, el)
        else tabRefs.current.delete(id)
      },
    []
  )
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>): void => {
      if (tabs.length === 0) return
      const currentIdx = tabs.findIndex((t) => t.id === activeId)
      if (currentIdx < 0) return
      let nextIdx: number | null = null
      switch (event.key) {
        case 'ArrowRight':
          nextIdx = (currentIdx + 1) % tabs.length
          break
        case 'ArrowLeft':
          nextIdx = (currentIdx - 1 + tabs.length) % tabs.length
          break
        case 'Home':
          nextIdx = 0
          break
        case 'End':
          nextIdx = tabs.length - 1
          break
        default:
          return
      }
      if (nextIdx === null) return
      event.preventDefault()
      const next = tabs[nextIdx].id
      onActiveChange(next)
      focusTabAt(tabRefs, next)
    },
    [activeId, onActiveChange, tabs]
  )

  return (
    <div className={`tabs${className ? ` ${className}` : ''}`}>
      <div
        role="tablist"
        aria-label={ariaLabel}
        className="tabs-tablist"
        tabIndex={-1}
        onKeyDown={handleKeyDown}
      >
        {tabs.map((tab) => {
          const selected = tab.id === activeId
          return (
            <button
              key={tab.id}
              ref={setTabRef(tab.id)}
              type="button"
              role="tab"
              id={`tab-${tab.id}`}
              aria-selected={selected}
              aria-controls={`tabpanel-${tab.id}`}
              tabIndex={selected ? 0 : -1}
              className={`tabs-tab${selected ? ' tabs-tab--active' : ''}`}
              data-test={tab.tabTestId}
              onClick={() => onActiveChange(tab.id)}
            >
              {tab.label}
            </button>
          )
        })}
      </div>
      {tabs.map((tab) => {
        const selected = tab.id === activeId
        return (
          <div
            key={tab.id}
            role="tabpanel"
            id={`tabpanel-${tab.id}`}
            aria-labelledby={`tab-${tab.id}`}
            className={`tabs-panel${panelClassName ? ` ${panelClassName}` : ''}`}
            data-test={tab.panelTestId}
            hidden={!selected}
          >
            {renderPanel(tab.id)}
          </div>
        )
      })}
    </div>
  )
}

export default Tabs
