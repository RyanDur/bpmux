import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { storyTypeFromLabels, type BoardStory } from './types'

interface Props {
  stories: BoardStory[]
  open: boolean
  onToggle: (open: boolean) => void
  onOpen: (n: number) => void
}

const UnscheduledList = ({ stories, open, onToggle, onOpen }: Props): React.JSX.Element | null => {
  if (stories.length === 0) return null
  return (
    <li key="unscheduled" className="board-iteration-item">
      <details
        className="board-iteration-card board-iteration-card--unscheduled"
        data-test="board-iteration-unscheduled"
        data-iteration-kind="unscheduled"
        open={open}
        onToggle={(event) => {
          onToggle(event.currentTarget.open)
        }}
      >
        <summary className="board-iteration-summary disclosure-summary">
          <b className="board-iteration-title">Unscheduled</b>
          <small className="board-iteration-count">({stories.length})</small>
        </summary>
        <ul className="board-iteration-body">
          {stories.map((story) => (
            <li
              key={story.number}
              className="board-iteration-member"
              data-test={`board-story-${story.number}`}
              data-iteration-member={story.number}
            >
              <button
                type="button"
                className="board-iteration-member-navigate board-story-navigate"
                aria-label={`open #${story.number} ${story.title}`}
                onClick={() => onOpen(story.number)}
              />
              <small className="board-story-number">#{story.number}</small>
              <StoryTypeGlyph type={storyTypeFromLabels(story.labels)} />
              <StoryBadge story={story} onOpenBlocker={onOpen} />
              <b className="board-story-title" data-test="story-title">
                {story.title}
              </b>
              <ReworkGlyph reworkComment={story.rework_comment} />
              {typeof story.parent_epic === 'number' ? (
                <button
                  type="button"
                  className="board-story-epic-ref"
                  data-test="epic-link"
                  aria-label={`open epic #${story.parent_epic}`}
                  onClick={(event) => {
                    event.preventDefault()
                    event.stopPropagation()
                    onOpen(story.parent_epic as number)
                  }}
                >
                  ↳ Epic #{story.parent_epic}
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      </details>
    </li>
  )
}

export default UnscheduledList
