import StoryRow from './StoryRow'
import { DRAG_MIMETYPE_REGISTRY } from './useStoryDrag'
import { useDragReorder } from './useDragReorder'
import { isClosedIterationMarker, type BoardStory, type ColumnState, type GroupSpec } from './types'

const decodeStoryNumber = (raw: string): number | null => {
  const n = Number.parseInt(raw, 10)
  return Number.isNaN(n) ? null : n
}

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

  const { orderedItems, draggedNumber, rowProps, gripProps } = useDragReorder<number, BoardStory>({
    items: stories,
    keyOf: (s) => s.number,
    mimeType: DRAG_MIMETYPE_REGISTRY.STORY_ROW,
    encode: String,
    decode: decodeStoryNumber,
    payloadKey: (p) => p,
    buildPayload: (s) => s.number,
    disabled: !draggable,
    onReorder: (moved, targetNumber) => {
      if (onReorder === undefined) return
      onReorder(group.state, moved, targetNumber)
    }
  })

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
          <ul className="board-story-list">
            {orderedItems.map((story, i) => {
              const rowDraggable = draggable && !isClosedIterationMarker(story)
              return (
                <StoryRow
                  key={story.number}
                  story={story}
                  parentEpicTitle={
                    story.parent_epic !== undefined
                      ? epicTitleByNumber.get(story.parent_epic)
                      : undefined
                  }
                  selected={story.number === selectedNumber}
                  draggable={rowDraggable}
                  dragging={draggedNumber === story.number}
                  position={i + 1}
                  total={orderedItems.length}
                  onKeyboardReorder={
                    rowDraggable && onReorder !== undefined
                      ? (direction) => {
                          if (direction === 'up' && i === 0) return
                          if (direction === 'down' && i === orderedItems.length - 1) return
                          const target =
                            direction === 'up' ? orderedItems[i - 1] : orderedItems[i + 1]
                          onReorder(group.state, story.number, target.number)
                        }
                      : undefined
                  }
                  onOpen={onOpen}
                  onOpenTimeline={onOpenTimeline}
                  onSelect={onSelect}
                  rowDragProps={rowDraggable ? rowProps(story) : undefined}
                  gripDragProps={rowDraggable ? gripProps(story) : undefined}
                />
              )
            })}
          </ul>
        )}
      </section>
    </details>
  )
}

export default BoardGroup
