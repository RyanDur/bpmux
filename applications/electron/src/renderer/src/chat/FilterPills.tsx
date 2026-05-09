import {
  MESSAGE_CLASSES,
  type MessageClass,
  type MessageFilter
} from '../hooks/useMessageFilter'
import './FilterPills.css'

interface Props {
  filter: MessageFilter
  // Hidden-count per filtered-out class. Keyed by class; absent means
  // the class is visible or has zero hidden messages.
  hiddenCounts: Partial<Record<MessageClass, number>>
}

function hiddenSummary(counts: Partial<Record<MessageClass, number>>): string | null {
  // Class names render as invariants ("1 activity, 3 system hidden"),
  // not English words — pluralizing "activity → activitys" reads broken
  // and "activity → activities" invents a lexical relationship the UI
  // doesn't mean. Spec body wording: '12 activity, 3 system hidden'.
  // Render order follows MESSAGE_CLASSES so the summary is stable.
  const parts: string[] = []
  for (const cls of MESSAGE_CLASSES) {
    const n = counts[cls] ?? 0
    if (n > 0) parts.push(`${n} ${cls}`)
  }
  if (parts.length === 0) return null
  return `${parts.join(', ')} hidden`
}

function FilterPills({ filter, hiddenCounts }: Props): React.JSX.Element {
  const { visible, toggle, setAll, isAllActive } = filter
  const summary = hiddenSummary(hiddenCounts)

  return (
    <aside
      className="filter-pills"
      data-test="filter-pills"
      aria-label="Message class filter"
    >
      <div className="filter-pills-row">
        <button
          type="button"
          className={`filter-pill${isAllActive ? ' filter-pill--active' : ''}`}
          aria-pressed={isAllActive}
          data-test="filter-pill-all"
          onClick={setAll}
        >
          all
        </button>
        {MESSAGE_CLASSES.map((cls) => {
          const active = visible.has(cls)
          return (
            <button
              key={cls}
              type="button"
              className={`filter-pill${active ? ' filter-pill--active' : ''}`}
              aria-pressed={active}
              data-test={`filter-pill-${cls}`}
              onClick={() => toggle(cls)}
            >
              {cls}
            </button>
          )
        })}
      </div>
      {summary && (
        <output
          className="filter-pills-summary"
          aria-live="polite"
          data-test="filter-hidden-count"
        >
          {summary}
        </output>
      )}
    </aside>
  )
}

export default FilterPills
