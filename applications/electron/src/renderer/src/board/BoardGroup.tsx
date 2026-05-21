import { useState } from 'react'
import StoryRow from './StoryRow'
import { DRAG_MIMETYPE_REGISTRY, useStoryDrag } from './useStoryDrag'
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
  onReorder?: (state: ColumnState, moved: number, insertAt: number) => void
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
  const [dropAt, setDropAt] = useState<number | null>(null)
  const {
    draggedNumber: draggingNumber,
    startDrag,
    endDrag,
    decode
  } = useStoryDrag<number>({
    mimeType: DRAG_MIMETYPE_REGISTRY.STORY_ROW,
    encode: String,
    decode: decodeStoryNumber
  })

  const handleListDragOver = (event: React.DragEvent<HTMLElement>): void => {
    if (!draggable) return
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }

  const handleRowDragOver =
    (index: number) =>
    (event: React.DragEvent<HTMLLIElement>): void => {
      if (!draggable) return
      event.preventDefault()
      const rect = event.currentTarget.getBoundingClientRect()
      const mid = rect.top + rect.height / 2
      setDropAt(event.clientY < mid ? index : index + 1)
    }

  const handleDrop = (event: React.DragEvent<HTMLElement>): void => {
    if (!draggable || onReorder === undefined) return
    event.preventDefault()
    const moved = decode(event)
    if (moved === null || dropAt === null) {
      setDropAt(null)
      endDrag()
      return
    }
    onReorder(group.state, moved, dropAt)
    setDropAt(null)
    endDrag()
  }

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
        className="board-group-summary"
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
          <ul
            className="board-story-list"
            onDragOver={handleListDragOver}
            onDrop={handleDrop}
            onDragLeave={() => setDropAt(null)}
          >
            {stories.map((story, i) => {
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
                  dragging={draggingNumber === story.number}
                  dropBefore={dropAt === i}
                  dropAfterLast={dropAt === stories.length && i === stories.length - 1}
                  position={i + 1}
                  total={stories.length}
                  onKeyboardReorder={
                    rowDraggable && onReorder !== undefined
                      ? (direction) => {
                          if (direction === 'up' && i === 0) return
                          if (direction === 'down' && i === stories.length - 1) return
                          const insertAt =
                            direction === 'up'
                              ? Math.max(0, i - 1)
                              : Math.min(stories.length, i + 2)
                          onReorder(group.state, story.number, insertAt)
                        }
                      : undefined
                  }
                  onOpen={onOpen}
                  onOpenTimeline={onOpenTimeline}
                  onSelect={onSelect}
                  onRowDragOver={handleRowDragOver(i)}
                  onDragStartRow={(event) => startDrag(event, story.number, story.number)}
                  onDragEndRow={() => {
                    endDrag()
                    setDropAt(null)
                  }}
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
