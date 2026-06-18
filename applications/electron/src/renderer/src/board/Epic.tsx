import { useState } from 'react'
import MiniBoard from './MiniBoard'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useEpicCollapse } from './useEpicCollapse'
import { RENDERER_IDENTITY } from '../transport/session'
import { deriveEpicState, type BoardStory } from './types'
import {
  breakdownAriaLabel,
  breakdownFor,
  extractDescription,
  isOpenByDefault,
  progressDenominatorFor,
  sortEpics,
  stripEpicPrefix,
  type EpicSort
} from './epicSummary'

interface Props {
  epics: BoardStory[]
  stories: BoardStory[]
  onOpen: (n: number) => void
}

const Epic = ({ epics, stories, onOpen }: Props): React.JSX.Element | null => {
  const collapse = useEpicCollapse(RENDERER_IDENTITY)
  const [sort, setSort] = useState<EpicSort>('progress')
  if (epics.length === 0) return null
  const sortedEpics = sortEpics(epics, sort, stories)
  return (
    <section className="board-epics" data-test="board-epics">
      <header className="board-epics-header">
        <b className="board-group-label">Epics</b>
        <small className="board-group-count">{epics.length}</small>
        <button
          type="button"
          className="board-epics-sort"
          data-test="board-epics-sort"
          data-sort={sort}
          aria-label={`Sort epics by ${sort === 'progress' ? 'progress' : 'created date'}; click to change`}
          onClick={() => setSort((prev) => (prev === 'progress' ? 'created' : 'progress'))}
        >
          sort: {sort === 'progress' ? 'progress' : 'created'}
        </button>
      </header>
      <ul className="board-epics-list">
        {sortedEpics.map((epic) => {
          const breakdown = breakdownFor(epic, stories)
          const children = stories.filter((s) => s.parent_epic === epic.number)
          const derived = deriveEpicState(epic.state, children)
          const description = extractDescription(epic.body)
          const defaultOpen = isOpenByDefault(derived)
          const open = collapse.isOpen(epic.number, defaultOpen)
          const denom = progressDenominatorFor(breakdown)
          const displayTitle = stripEpicPrefix(epic.title)
          return (
            <li key={epic.number} className="board-epic-item">
              <details
                className="board-epic-card"
                data-test={`board-epic-${epic.number}`}
                open={open}
                onToggle={(event) => {
                  collapse.setOpen(epic.number, event.currentTarget.open)
                }}
              >
                <summary className="board-epic-summary disclosure-summary">
                  <StoryTypeGlyph type="epic" />
                  <small className="board-story-number">#{epic.number}</small>
                  <output
                    className="board-epic-state"
                    data-state={derived}
                    data-test={`board-epic-${epic.number}-state`}
                  >
                    {derived}
                  </output>
                  <b className="board-epic-title">{displayTitle}</b>
                  <progress
                    className="board-epic-progress board-epic-progress--summary"
                    data-test={`board-epic-${epic.number}-progress`}
                    value={breakdown.accepted}
                    max={denom === 0 ? 1 : denom}
                    aria-label={breakdownAriaLabel(breakdown)}
                  />
                  <small
                    className="board-epic-summary-progress"
                    data-test={`board-epic-${epic.number}-summary-progress`}
                    aria-label={breakdownAriaLabel(breakdown)}
                  >
                    {denom === 0 ? (
                      <i
                        className="board-epic-count board-epic-count--empty"
                        data-test={`board-epic-${epic.number}-count-empty`}
                      >
                        empty
                      </i>
                    ) : (
                      <output
                        className="board-epic-count"
                        data-test={`board-epic-${epic.number}-count`}
                      >
                        {`${breakdown.accepted} / ${denom}`}
                      </output>
                    )}
                  </small>
                </summary>
                <section className="board-epic-body">
                  {description !== '' && (
                    <p
                      className="board-epic-description"
                      data-test={`board-epic-${epic.number}-description`}
                    >
                      {description}
                    </p>
                  )}
                  <MiniBoard epicNumber={epic.number} stories={children} />
                  <button
                    type="button"
                    className="board-epic-view-details"
                    data-test={`board-epic-${epic.number}-view-details`}
                    onClick={() => onOpen(epic.number)}
                    aria-label={`view epic details: #${epic.number} ${displayTitle}`}
                  >
                    view epic details →
                  </button>
                </section>
              </details>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

export default Epic
