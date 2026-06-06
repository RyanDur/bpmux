import StoryRow from './StoryRow'
import { SortableList } from './SortableList'
import { READY_STORY_DRAG } from './sortableDragConfigs'
import { isClosedIterationMarker, type BoardStory, type ColumnState, type GroupSpec } from './types'

export interface BoardGroupProps {
  group: GroupSpec
  stories: BoardStory[]
  epicTitleByNumber: Map<number, string>
  loading: boolean
  selectedNumber: number | null
  onOpen: (n: number) => void
  onOpenTimeline?: (n: number) => void
  onSelect: (n: number) => void
  onReorder?: (state: ColumnState, moved: number, targetNumber: number) => void
  collapsed: boolean
  onToggleCollapse: () => void
  children?: React.ReactNode
}

const BoardGroup = ({
  group,
  stories,
  epicTitleByNumber,
  loading,
  selectedNumber,
  onOpen,
  onOpenTimeline,
  onSelect,
  onReorder,
  collapsed,
  onToggleCollapse,
  children
}: BoardGroupProps): React.JSX.Element => {
  const draggable = (group.state === 'ready' || group.state === 'icebox') && onReorder !== undefined

  const contentId = `board-group-${group.state}-content`
  return (
    <details
      className="board-group"
      open={!collapsed}
      onToggle={(e) => {
        const nextOpen = (e.currentTarget as HTMLDetailsElement).open
        if (nextOpen === collapsed) onToggleCollapse()
      }}
    >
      <summary
        className="board-group-summary disclosure-summary"
        data-test={`board-group-${group.state}`}
        aria-expanded={!collapsed}
        aria-controls={contentId}
      >
        <i className="board-group-chevron" aria-hidden="true" />
        <small
          className="board-group-count"
          data-test={`board-group-${group.state}-count`}
          data-empty={stories.length === 0 ? 'true' : undefined}
        >
          ({stories.length})
        </small>
        <b className="board-group-label">{group.label}</b>
      </summary>
      <section id={contentId}>
        {loading ? (
          <ul className="board-story-list board-story-list--skeleton" aria-hidden="true">
            {[0, 1, 2].map((i) => (
              <li key={i} className="board-story-row board-story-row--skeleton">
                <i className="skeleton-bar" />
              </li>
            ))}
          </ul>
        ) : children !== undefined ? (
          children
        ) : stories.length === 0 ? (
          <p className="board-group-empty">none</p>
        ) : (
          <SortableList<number, BoardStory>
            items={stories}
            keyOf={(s) => s.number}
            rowKey={(s) => s.number}
            config={READY_STORY_DRAG}
            buildPayload={(s) => s.number}
            onReorder={(moved, targetNumber) => {
              if (onReorder === undefined) return
              onReorder(group.state, moved, targetNumber)
            }}
            disabled={!draggable}
            ulClassName="board-story-list"
            renderRow={(story, { rowDragProps, gripDragProps, dragging, index }) => {
              const rowDraggable = draggable && !isClosedIterationMarker(story)
              return (
                <StoryRow
                  story={story}
                  parentEpicTitle={
                    story.parent_epic !== undefined
                      ? epicTitleByNumber.get(story.parent_epic)
                      : undefined
                  }
                  selected={story.number === selectedNumber}
                  draggable={rowDraggable}
                  dragging={dragging}
                  position={index + 1}
                  total={stories.length}
                  onKeyboardReorder={
                    rowDraggable && onReorder !== undefined
                      ? (direction) => {
                          if (direction === 'up' && index === 0) return
                          if (direction === 'down' && index === stories.length - 1) return
                          const target = direction === 'up' ? stories[index - 1] : stories[index + 1]
                          onReorder(group.state, story.number, target.number)
                        }
                      : undefined
                  }
                  onOpen={onOpen}
                  onOpenTimeline={onOpenTimeline}
                  onSelect={onSelect}
                  rowDragProps={rowDraggable ? rowDragProps : undefined}
                  gripDragProps={rowDraggable ? gripDragProps : undefined}
                />
              )
            }}
          />
        )}
      </section>
    </details>
  )
}

export default BoardGroup
