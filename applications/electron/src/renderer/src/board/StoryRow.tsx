import { GripIcon } from './GripIcon'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import { deriveBadge } from './StoryBadge.utils'
import StoryTypeGlyph from './StoryTypeGlyph'
import { STORY_GRIP_TEST_ID } from './testIds'
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
  onDragStartRow?: (event: React.DragEvent<HTMLElement>) => void
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
      data-story-type={type}
      data-closed={story.gh_state === 'closed' ? 'true' : undefined}
      data-dragging={dragging ? 'true' : undefined}
      onDragOver={onRowDragOver}
    >
      <button
        type="button"
        className="board-story-navigate"
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
            }
          }
        }}
        title={story.title}
        aria-label={ariaLabel}
        data-rework={story.rework_comment ? 'true' : undefined}
      />
      {draggable ? (
        <button
          type="button"
          className="board-story-grip"
          data-test={STORY_GRIP_TEST_ID}
          draggable={true}
          aria-label={`Drag to reorder #${story.number}`}
          tabIndex={-1}
          onDragStart={(event) => onDragStartRow?.(event)}
          onDragEnd={() => onDragEndRow?.()}
        >
          <GripIcon />
        </button>
      ) : null}
      <small className="board-story-number">#{story.number}</small>
      <StoryTypeGlyph type={type} />
      <StoryBadge story={story} onOpenTimeline={onOpenTimeline} onOpenBlocker={onOpen} />
      <b className="board-story-title" data-test="story-title">
        {story.title}
      </b>
      <ReworkGlyph reworkComment={story.rework_comment} />
      {typeof parentEpic === 'number' ? (
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
    </li>
  )
}

export default StoryRow
