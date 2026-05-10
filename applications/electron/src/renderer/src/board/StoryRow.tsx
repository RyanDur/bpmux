import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge, { deriveBadge } from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { storyTypeFromLabels, type BoardStory } from './types'

export interface StoryRowProps {
  story: BoardStory
  parentEpicTitle?: string
  selected: boolean
  draggable: boolean
  dragging: boolean
  dropBefore: boolean
  dropAfterLast: boolean
  position: number
  total: number
  onKeyboardReorder?: (direction: 'up' | 'down') => void
  onOpen: (n: number) => void
  onOpenTimeline?: (n: number) => void
  onSelect: (n: number) => void
  onRowDragOver: (event: React.DragEvent<HTMLLIElement>) => void
  onDragStartRow?: () => void
  onDragEndRow?: () => void
}

const StoryRow = ({
  story,
  parentEpicTitle,
  selected,
  draggable,
  dragging,
  dropBefore,
  dropAfterLast,
  position,
  total,
  onKeyboardReorder,
  onOpen,
  onOpenTimeline,
  onSelect,
  onRowDragOver,
  onDragStartRow,
  onDragEndRow
}: StoryRowProps): React.JSX.Element => {
  const type = storyTypeFromLabels(story.labels)
  const classes = ['board-story-row']
  if (dropBefore) classes.push('board-story-row--drop-before')
  if (dropAfterLast) classes.push('board-story-row--drop-after')
  if (deriveBadge(story) === 'blocked') classes.push('board-story-row--blocked')
  const reorderHint =
    onKeyboardReorder !== undefined
      ? ' — press Ctrl+Shift+Up to move up, Ctrl+Shift+Down to move down'
      : ''
  const ariaLabel = `Story #${story.number} ${story.title}, position ${position} of ${total}${reorderHint}`
  const parentEpic = story.parent_epic
  const epicAriaLabel =
    parentEpicTitle !== undefined
      ? `open epic #${parentEpic} ${parentEpicTitle}`
      : `open epic #${parentEpic}`
  return (
    <li
      className={classes.join(' ')}
      aria-current={selected ? 'true' : undefined}
      data-test={`board-story-${story.number}`}
      data-dragging={dragging ? 'true' : undefined}
      draggable={draggable}
      onDragStart={(event) => {
        if (!draggable) return
        event.dataTransfer.setData('text/bpmux-story', String(story.number))
        event.dataTransfer.effectAllowed = 'move'
        onDragStartRow?.()
      }}
      onDragEnd={() => onDragEndRow?.()}
      onDragOver={onRowDragOver}
    >
      <div
        role="button"
        tabIndex={0}
        className="board-story-button"
        onClick={() => onOpen(story.number)}
        onFocus={() => onSelect(story.number)}
        onMouseEnter={() => onSelect(story.number)}
        onKeyDown={(event) => {
          if (event.ctrlKey && event.shiftKey && onKeyboardReorder !== undefined) {
            if (event.key === 'ArrowUp') {
              event.preventDefault()
              onKeyboardReorder('up')
              return
            }
            if (event.key === 'ArrowDown') {
              event.preventDefault()
              onKeyboardReorder('down')
              return
            }
          }
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault()
            onOpen(story.number)
          }
        }}
        title={story.title}
        aria-label={ariaLabel}
        data-rework={story.rework_comment ? 'true' : undefined}
      >
        <small className="board-story-number">#{story.number}</small>
        <StoryTypeGlyph type={type} />
        <StoryBadge story={story} onOpenTimeline={onOpenTimeline} onOpenBlocker={onOpen} />
        <b className="board-story-title" data-test="story-title">
          {story.title}
        </b>
        <ReworkGlyph reworkComment={story.rework_comment} />
        {parentEpic !== undefined ? (
          <button
            type="button"
            className="board-story-epic-ref"
            data-test="epic-link"
            aria-label={epicAriaLabel}
            onClick={(event) => {
              event.preventDefault()
              event.stopPropagation()
              onOpen(parentEpic)
            }}
          >
            ↳ Epic #{parentEpic}
          </button>
        ) : null}
      </div>
    </li>
  )
}

export default StoryRow
