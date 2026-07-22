import { useState } from 'react'
import MiniBoard from './MiniBoard'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useEpicCollapse } from './useEpicCollapse'
import { RENDERER_IDENTITY } from '../state'
import { deriveEpicState, type Epic as EpicData, type Story } from './types'
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
  epics: EpicData[]
  stories: Story[]
  onOpen: (n: number) => void
}

const Epic = ({ epics, stories, onOpen }: Props): React.JSX.Element | null => {
  const collapse = useEpicCollapse(RENDERER_IDENTITY)
  const [sort, setSort] = useState<EpicSort>('progress')
  if (epics.length === 0) return null
  const sortedEpics = sortEpics(epics, sort, stories)
  return (
    <section className="board-epics" data-test="board-epics" aria-label="Epics">
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
          const children = stories.filter((s) => s.parent_epic === epic.id)
          const derived = deriveEpicState(epic.state, children)
          const description = extractDescription(epic.body)
          const defaultOpen = isOpenByDefault(derived)
          const open = collapse.isOpen(epic.id, defaultOpen)
          const denom = progressDenominatorFor(breakdown)
          const displayTitle = stripEpicPrefix(epic.title)
          return (
            <li key={epic.id} className="board-epic-item">
              <details
                className="board-epic-card"
                data-test={`board-epic-${epic.id}`}
                aria-label={`Epic #${epic.id} ${displayTitle}`}
                open={open}
                onToggle={(event) => {
                  collapse.setOpen(epic.id, event.currentTarget.open)
                }}
              >
                <summary className="board-epic-summary disclosure-summary">
                  <StoryTypeGlyph type="epic" />
                  <small className="board-story-number">#{epic.id}</small>
                  <output
                    className="board-epic-state"
                    data-state={derived}
                    data-test={`board-epic-${epic.id}-state`}
                  >
                    {derived}
                  </output>
                  <b className="board-epic-title">{displayTitle}</b>
                  <progress
                    className="board-epic-progress board-epic-progress--summary"
                    data-test={`board-epic-${epic.id}-progress`}
                    value={breakdown.accepted}
                    max={denom === 0 ? 1 : denom}
                    aria-label={breakdownAriaLabel(breakdown)}
                  />
                  <small
                    className="board-epic-summary-progress"
                    data-test={`board-epic-${epic.id}-summary-progress`}
                    aria-label={breakdownAriaLabel(breakdown)}
                  >
                    {denom === 0 ? (
                      <i
                        className="board-epic-count board-epic-count--empty"
                        data-test={`board-epic-${epic.id}-count-empty`}
                      >
                        empty
                      </i>
                    ) : (
                      <output
                        className="board-epic-count"
                        data-test={`board-epic-${epic.id}-count`}
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
                      data-test={`board-epic-${epic.id}-description`}
                    >
                      {description}
                    </p>
                  )}
                  <MiniBoard epicId={epic.id} stories={children} />
                  <button
                    type="button"
                    className="board-epic-view-details"
                    data-test={`board-epic-${epic.id}-view-details`}
                    onClick={() => onOpen(epic.id)}
                    aria-label={`view epic details: #${epic.id} ${displayTitle}`}
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
