// #1302 rework — sister-pattern to Epic.tsx. Renders iteration-issues
// (story_type=Iteration) as parent-rows in a section, with member-
// stories (iteration_id === iteration.number) grouped under each.
//
// Visual treatment mirrors Epic+children: collapsible details/summary
// per iteration; member-stories shown via a sister-MiniBoard so the
// grouping affordance reads consistently with Epic-grouping. Closed
// iterations stay in retired view per the columnFor / isActiveIteration
// filter — this component renders ACTIVE iterations only.

import MiniBoard from './MiniBoard'
import StoryTypeGlyph from './StoryTypeGlyph'
import { type BoardStory } from './types'

interface Props {
  iterations: BoardStory[]
  stories: BoardStory[]
  onOpen: (n: number) => void
}

const Iteration = ({ iterations, stories, onOpen }: Props): React.JSX.Element | null => {
  if (iterations.length === 0) return null
  return (
    <section className="board-iterations" data-test="board-iterations">
      <header className="board-iterations-header">
        <b className="board-group-label">Iterations</b>
        <small className="board-group-count">{iterations.length}</small>
      </header>
      <ul className="board-iterations-list">
        {iterations.map((iteration) => {
          const members = stories.filter((s) => s.iteration_id === iteration.number)
          const open = members.length > 0
          return (
            <li key={iteration.number} className="board-iteration-item">
              <details
                className="board-iteration-card"
                data-test={`board-iteration-${iteration.number}`}
                open={open}
              >
                <summary className="board-iteration-summary">
                  <button
                    type="button"
                    className="board-iteration-open-button"
                    onClick={(event) => {
                      event.preventDefault()
                      event.stopPropagation()
                      onOpen(iteration.number)
                    }}
                    title={iteration.title}
                    aria-label={`open iteration #${iteration.number} ${iteration.title}`}
                  >
                    <StoryTypeGlyph type="iteration" />
                    <small className="board-story-number">#{iteration.number}</small>
                    <b
                      className="board-iteration-title"
                      data-test={`board-iteration-${iteration.number}-title`}
                    >
                      {iteration.title}
                    </b>
                    <small
                      className="board-iteration-count"
                      data-test={`board-iteration-${iteration.number}-count`}
                    >
                      ({members.length})
                    </small>
                  </button>
                </summary>
                <div className="board-iteration-body">
                  <MiniBoard epicNumber={iteration.number} stories={members} />
                </div>
              </details>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

export default Iteration
