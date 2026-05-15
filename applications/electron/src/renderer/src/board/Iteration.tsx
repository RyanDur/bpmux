import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { storyTypeFromLabels, type BoardStory } from './types'

interface Props {
  stories: BoardStory[]
  onOpen: (n: number) => void
}

const UNSCHEDULED_KEY = 'unscheduled'

const isIterationStory = (story: BoardStory): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

const isLooseIceboxStory = (story: BoardStory): boolean => {
  if (isIterationStory(story)) return false
  if (story.state !== 'icebox') return false
  if (story.labels.some((l) => l.startsWith('iteration-'))) return false
  return true
}

const Iteration = ({ stories, onOpen }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const iterations = stories.filter(isIterationStory)
  const loose = stories.filter(isLooseIceboxStory)
  if (iterations.length === 0 && loose.length === 0) return null

  return (
    <section className="board-iterations" data-test="board-iterations">
      <ul className="board-iterations-list">
        {iterations.map((iteration) => {
          const key = String(iteration.number)
          const defaultOpen = iteration.gh_state !== 'closed'
          const open = collapse.isOpen(key, defaultOpen)
          return (
            <li key={iteration.number} className="board-iteration-item">
              <details
                className="board-iteration-card"
                data-test={`board-iteration-${iteration.number}`}
                data-iteration-kind="named"
                data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
                open={open}
                onToggle={(event) => {
                  collapse.setOpen(key, event.currentTarget.open)
                }}
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
                    <b className="board-iteration-title">{iteration.title}</b>
                  </button>
                </summary>
                <ul className="board-iteration-body"></ul>
              </details>
            </li>
          )
        })}
        {loose.length > 0 && (
          <li key="unscheduled" className="board-iteration-item">
            <details
              className="board-iteration-card board-iteration-card--unscheduled"
              data-test="board-iteration-unscheduled"
              data-iteration-kind="unscheduled"
              open={collapse.isOpen(UNSCHEDULED_KEY, true)}
              onToggle={(event) => {
                collapse.setOpen(UNSCHEDULED_KEY, event.currentTarget.open)
              }}
            >
              <summary className="board-iteration-summary">
                <b className="board-iteration-title">Unscheduled</b>
                <small className="board-iteration-count">{loose.length}</small>
              </summary>
              <ul className="board-iteration-body">
                {loose.map((story) => (
                  <li
                    key={story.number}
                    className="board-iteration-member"
                    data-test={`board-story-${story.number}`}
                  >
                    <button
                      type="button"
                      className="board-iteration-member-button"
                      onClick={() => onOpen(story.number)}
                    >
                      <small className="board-story-number">#{story.number}</small>
                      <span className="board-story-title">{story.title}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </details>
          </li>
        )}
      </ul>
    </section>
  )
}

export default Iteration
