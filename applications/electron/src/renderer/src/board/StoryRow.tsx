import { DragGrip } from './DragGrip'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { type GripDragProps, type RowDragProps } from './useDragReorder'
import { storyNavigateLabel } from './storyNavigateLabel'
import { isConcluded, storyTypeFromLabels, type Story } from './types'

interface StoryRowProps {
  story: Story
  parentEpicTitle?: string
  selected: boolean
  draggable: boolean
  dragging: boolean
  position: number
  total: number
  onKeyboardReorder?: (direction: 'up' | 'down') => void
  onOpen: (n: number) => void
  onOpenTimeline?: (n: number) => void
  onSelect: (n: number) => void
  rowDragProps?: RowDragProps
  gripDragProps?: GripDragProps
}

const StoryRow = ({
  story,
  parentEpicTitle,
  selected,
  draggable,
  dragging,
  position,
  total,
  onKeyboardReorder,
  onOpen,
  onOpenTimeline,
  onSelect,
  rowDragProps,
  gripDragProps
}: StoryRowProps): React.JSX.Element => {
  const type = storyTypeFromLabels(story.labels)
  const classes = ['board-story-row']
  const reorderHint =
    onKeyboardReorder !== undefined
      ? ' — press Ctrl+Shift+Up to move up, Ctrl+Shift+Down to move down'
      : ''
  const ariaLabel = `${storyNavigateLabel(story)}, position ${position} of ${total}${reorderHint}`
  const parentEpic = story.parent_epic
  const epicAriaLabel =
    parentEpicTitle !== undefined
      ? `open epic #${parentEpic} ${parentEpicTitle}`
      : `open epic #${parentEpic}`
  return (
    <li
      className={classes.join(' ')}
      aria-current={selected ? 'true' : undefined}
      data-test={`board-story-${story.id}`}
      data-story-type={type}
      data-closed={isConcluded(story.state) ? 'true' : undefined}
      data-dragging={dragging ? 'true' : undefined}
      {...rowDragProps}
    >
      <button
        type="button"
        className="board-story-navigate"
        tabIndex={selected ? 0 : -1}
        onClick={() => onOpen(story.id)}
        onFocus={() => onSelect(story.id)}
        onMouseEnter={() => onSelect(story.id)}
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
      {draggable && gripDragProps !== undefined ? (
        <DragGrip storyId={story.id} gripDragProps={gripDragProps} />
      ) : null}
      <small className="board-story-number">#{story.id}</small>
      <StoryTypeGlyph type={type} />
      <StoryBadge story={story} onOpenTimeline={onOpenTimeline} />
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
