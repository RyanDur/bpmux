import StoryTypeGlyph from './StoryTypeGlyph'
import type { BoardStory } from './types'

interface Props {
  epics: BoardStory[]
  stories: BoardStory[]
  onOpen: (n: number) => void
}

interface ChildCounts {
  total: number
  accepted: number
  delivered: number
  inProgress: number
}

const countChildren = (epic: BoardStory, all: BoardStory[]): ChildCounts => {
  const children = all.filter((s) => s.parent_epic === epic.number)
  return {
    total: children.length,
    accepted: children.filter((s) => s.state === 'accepted').length,
    delivered: children.filter((s) => s.state === 'delivered').length,
    inProgress: children.filter((s) => s.state === 'in-progress').length
  }
}

const EpicSummary = ({ epics, stories, onOpen }: Props): React.JSX.Element | null => {
  if (epics.length === 0) return null
  return (
    <section className="board-epics" data-test="board-epics">
      <header className="board-epics-header">
        <b className="board-group-label">Epics</b>
        <small className="board-group-count">{epics.length}</small>
      </header>
      <ul className="board-epics-list">
        {epics.map((epic) => {
          const counts = countChildren(epic, stories)
          const children = stories.filter((s) => s.parent_epic === epic.number)
          return (
            <li key={epic.number} className="board-epic-item">
              <details className="board-epic-details" data-test={`board-epic-${epic.number}`}>
                <summary className="board-epic-summary">
                  <StoryTypeGlyph type="epic" />
                  <small className="board-story-number">#{epic.number}</small>
                  <b className="board-epic-title">{epic.title}</b>
                  <small className="board-epic-counts" aria-label={epicCountsAriaLabel(counts)}>
                    {counts.total} children · {counts.accepted}✓ {counts.delivered}↻{' '}
                    {counts.inProgress}…
                  </small>
                </summary>
                {children.length === 0 ? (
                  <p className="board-group-empty">no children yet</p>
                ) : (
                  <ul className="board-epic-children">
                    {children.map((child) => (
                      <li key={child.number} className="board-epic-child">
                        <button
                          type="button"
                          className="board-story-button"
                          onClick={() => onOpen(child.number)}
                          title={child.title}
                        >
                          <small className="board-story-number">#{child.number}</small>
                          <span className="board-epic-child-state" data-state={child.state}>
                            {child.state}
                          </span>
                          <b className="board-story-title">{child.title}</b>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </details>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

const epicCountsAriaLabel = (c: ChildCounts): string =>
  `${c.total} children: ${c.accepted} accepted, ${c.delivered} delivered, ${c.inProgress} in progress`

export default EpicSummary
