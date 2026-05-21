import { MESSAGE_CLASSES, type MessageClass, type MessageFilter } from '../hooks/useMessageFilter'
import './FilterPills.css'

interface Props {
  filter: MessageFilter
  hiddenCounts: Partial<Record<MessageClass, number>>
  visibleCounts: Partial<Record<MessageClass, number>>
}

const FilterPills = ({ filter, hiddenCounts, visibleCounts }: Props): React.JSX.Element => {
  const { visible, toggle } = filter
  const allMuted = visible.size === 0

  return (
    <aside className="filter-pills" data-test="filter-pills" aria-label="Message class filter">
      <div className="filter-pills-row">
        {MESSAGE_CLASSES.map((cls) => {
          const active = visible.has(cls)
          const count = active ? (visibleCounts[cls] ?? 0) : (hiddenCounts[cls] ?? 0)
          const empty = count === 0
          return (
            <button
              key={cls}
              type="button"
              role="switch"
              aria-checked={active}
              aria-label={`${cls} messages ${active ? 'shown' : 'hidden'}; ${count} message${count === 1 ? '' : 's'}`}
              className={`filter-pill${active ? ' filter-pill--on' : ' filter-pill--off'}${empty ? ' filter-pill--empty' : ''}`}
              data-test={`filter-pill-${cls}`}
              data-state={active ? 'on' : 'off'}
              data-empty={empty ? 'true' : undefined}
              onClick={() => toggle(cls)}
            >
              <span className="filter-pill-label">{cls}</span>
              <small className="filter-pill-count" data-test={`filter-pill-${cls}-count`}>
                ({count})
              </small>
            </button>
          )
        })}
      </div>
      {allMuted && (
        <output
          className="filter-pills-empty-hint"
          aria-live="polite"
          data-test="filter-pills-empty-hint"
        >
          All categories muted — click a pill to re-enable.
        </output>
      )}
    </aside>
  )
}

export default FilterPills
