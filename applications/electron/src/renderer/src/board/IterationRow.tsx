import { useState } from 'react'
import { GripIcon } from './GripIcon'
import StoryTypeGlyph from './StoryTypeGlyph'
import { iterationNameFromTitle } from './labelSchema'
import { ITERATION_GRIP_TEST_ID, boardIterationTestId } from './testIds'
import { DRAG_MIMETYPE_REGISTRY } from './useStoryDrag'
import type { SortableRowContext } from './SortableList'
import type { useIterationCollapse } from './useIterationCollapse'
import type { IterationMember } from './useIterationMembers'
import type { BoardIterationData } from './selectIterations'
import MemberList from './MemberList'

interface Props {
  iteration: BoardIterationData
  collapse: ReturnType<typeof useIterationCollapse>
  currentIterationNumber: number | null
  getMembersFor: (iterationNumber: number) => IterationMember[]
  setPending: (storyNumber: number, iterationNumber: number, position: number) => void
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
  onReorderByKeyboard: (iteration: BoardIterationData, direction: 'up' | 'down') => void
  drag?: SortableRowContext
}

const IterationRow = ({
  iteration,
  collapse,
  currentIterationNumber,
  getMembersFor,
  setPending,
  onOpen,
  onReorderAnnounce,
  onReorderByKeyboard,
  drag
}: Props): React.JSX.Element => {
  const [dragRejected, setDragRejected] = useState(false)
  const key = String(iteration.number)
  const iterationName = iterationNameFromTitle(iteration.title)
  const members = getMembersFor(iteration.number)
  const defaultOpen = !iteration.complete
  const open = collapse.isOpen(key, defaultOpen)
  const isCurrent = iteration.number === currentIterationNumber
  const draggable = drag !== undefined

  const rejectClosedDrop = (event: React.DragEvent<HTMLElement>): void => {
    if (
      event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER) ||
      event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF)
    ) {
      event.dataTransfer.dropEffect = 'none'
      setDragRejected(true)
    }
  }

  return (
    <li className="board-iteration-item">
      <article className="board-iteration-row" {...(drag ? drag.rowDragProps : {})}>
        <details
          className="board-iteration-card"
          data-test={boardIterationTestId(iteration.number)}
          aria-label={`Iteration #${iteration.number} ${iteration.title}`}
          data-iteration-kind="named"
          data-iteration-name={iterationName}
          data-closed={iteration.complete ? 'true' : undefined}
          data-current-iteration={isCurrent ? 'true' : undefined}
          data-dragging={drag?.dragging ? 'true' : undefined}
          data-drag-rejected={dragRejected ? 'true' : undefined}
          open={open}
          onDragOver={draggable ? undefined : rejectClosedDrop}
          onDragLeave={draggable ? undefined : () => setDragRejected(false)}
          onToggle={(event) => {
            collapse.setOpen(key, event.currentTarget.open)
          }}
        >
          <summary
            className="board-iteration-summary disclosure-summary"
            data-test="board-iteration-summary"
          >
            <StoryTypeGlyph type="iteration" />
            <small className="board-story-number">#{iteration.number}</small>
            <b className="board-iteration-title">{iteration.title}</b>
            {iteration.complete && <small className="board-iteration-closed-badge">(closed)</small>}
            <small className="board-iteration-count">({members.length})</small>
          </summary>
          <section className="board-iteration-body">
            <MemberList
              iteration={iteration}
              members={members}
              setPending={setPending}
              onOpen={onOpen}
              onReorderAnnounce={onReorderAnnounce}
            />
            <button
              type="button"
              className="board-iteration-view-details"
              data-test={`board-iteration-view-details-${iteration.number}`}
              onClick={() => onOpen(iteration.number)}
              aria-label={`view iteration details: #${iteration.number} ${iteration.title}`}
            >
              view iteration details →
            </button>
          </section>
        </details>
        {drag ? (
          <button
            type="button"
            className="board-iteration-grip"
            data-test={ITERATION_GRIP_TEST_ID}
            tabIndex={0}
            aria-label={`Reorder iteration #${iteration.number} ${iteration.title} — drag, or Ctrl+Shift+Up/Down`}
            aria-keyshortcuts="Control+Shift+ArrowUp Control+Shift+ArrowDown"
            {...drag.gripDragProps}
            onKeyDown={(event) => {
              if (!event.ctrlKey || !event.shiftKey) return
              const direction =
                event.key === 'ArrowUp' ? 'up' : event.key === 'ArrowDown' ? 'down' : null
              if (direction === null) return
              event.preventDefault()
              onReorderByKeyboard(iteration, direction)
            }}
          >
            <GripIcon />
          </button>
        ) : null}
      </article>
    </li>
  )
}

export default IterationRow
