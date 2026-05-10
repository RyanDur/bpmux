import { GROUP_ORDER, groupStories, type BoardStory } from './types'

interface Props {
  epicNumber: number
  stories: BoardStory[]
}

const MiniBoard = ({ epicNumber, stories }: Props): React.JSX.Element => {
  const grouped = groupStories(stories)
  return (
    <div className="board-epic-miniboard" data-test={`board-epic-${epicNumber}-miniboard`}>
      {GROUP_ORDER.map((group) => {
        const colStories = grouped.get(group.state) ?? []
        const colTest = `board-epic-${epicNumber}-miniboard-column-${group.state}`
        const childrenWord = colStories.length === 1 ? 'child' : 'children'
        return (
          <details
            key={group.state}
            className="board-epic-miniboard-column"
            data-test={colTest}
            data-state={group.state}
            open={colStories.length > 0}
          >
            <summary
              className="board-epic-miniboard-column-summary"
              aria-label={`${group.label}: ${colStories.length} ${childrenWord}`}
            >
              <b className="board-epic-miniboard-column-label">{group.label}</b>
              <small className="board-epic-miniboard-column-count" data-test={`${colTest}-count`}>
                ({colStories.length})
              </small>
            </summary>
            {colStories.length > 0 && (
              <ul className="board-epic-miniboard-children">
                {colStories.map((story) => (
                  <li
                    key={story.number}
                    className="board-epic-miniboard-child"
                    data-test={`board-epic-${epicNumber}-miniboard-child-${story.number}`}
                    aria-label={`#${story.number} ${story.title}, ${story.state}`}
                  >
                    <small className="board-epic-miniboard-child-number">#{story.number}</small>{' '}
                    <span className="board-epic-miniboard-child-title">{story.title}</span>
                  </li>
                ))}
              </ul>
            )}
          </details>
        )
      })}
    </div>
  )
}

export default MiniBoard
