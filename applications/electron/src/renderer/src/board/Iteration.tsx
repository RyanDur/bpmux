import { GripIcon } from './GripIcon'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { computeDropMutation } from './iterationPosition'
import { themeNameFromTitle } from './labelSchema'
import { ITERATION_GRIP_TEST_ID, boardIterationTestId } from './testIds'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import type { BoardStory } from './types'
import {
  DRAG_MIMETYPE,
  DRAG_MIMETYPE_ITERATION,
  type IterationDragPayload,
  positionInIterationOrder,
  postLabelMutation
} from './iterationMembers'
import { useIterationMembers } from './useIterationMembers'
import { useIterationDragState } from './useIterationDragState'
import MemberList from './MemberList'
import UnscheduledList from './UnscheduledList'

interface Props {
  stories: BoardStory[]
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const UNSCHEDULED_KEY = 'unscheduled'

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const {
    draggedNumber,
    setDraggedNumber,
    hoveredNumber,
    setHoveredNumber,
    draggedIterationNumber,
    setDraggedIterationNumber,
    dragRejectedIterationNumber,
    setDragRejectedIterationNumber
  } = useIterationDragState()
  const { sortedIterations, sortedLoose, getMembersFor, setPending } =
    useIterationMembers(stories)
  if (sortedIterations.length === 0 && sortedLoose.length === 0) return null

  return (
    <section className="board-iterations" data-test="board-iterations">
      <ul className="board-iterations-list">
        {sortedIterations.map((iteration) => {
          const key = String(iteration.number)
          const themeName = themeNameFromTitle(iteration.title)
          const members = getMembersFor(themeName)
          if (members.length === 0 && iteration.gh_state !== 'closed') return null
          const defaultOpen = iteration.gh_state !== 'closed'
          const open = collapse.isOpen(key, defaultOpen)
          return (
            <li key={iteration.number} className="board-iteration-item">
              <details
                className="board-iteration-card"
                data-test={boardIterationTestId(iteration.number)}
                data-iteration-kind="named"
                data-iteration-name={themeName}
                data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
                data-dragging={draggedIterationNumber === iteration.number ? 'true' : undefined}
                data-drag-rejected={
                  dragRejectedIterationNumber === iteration.number ? 'true' : undefined
                }
                open={open}
                onDragOver={(event) => {
                  if (iteration.gh_state === 'closed') {
                    if (
                      event.dataTransfer.types.includes(DRAG_MIMETYPE) ||
                      event.dataTransfer.types.includes(DRAG_MIMETYPE_ITERATION)
                    ) {
                      event.dataTransfer.dropEffect = 'none'
                      setDragRejectedIterationNumber(iteration.number)
                    }
                    return
                  }
                  if (event.dataTransfer.types.includes(DRAG_MIMETYPE_ITERATION)) {
                    event.preventDefault()
                    event.dataTransfer.dropEffect = 'move'
                  }
                }}
                onDragLeave={() => {
                  if (dragRejectedIterationNumber === iteration.number) {
                    setDragRejectedIterationNumber(null)
                  }
                }}
                onDrop={(event) => {
                  const raw = event.dataTransfer.getData(DRAG_MIMETYPE_ITERATION)
                  if (!raw) return
                  event.preventDefault()
                  const payload = JSON.parse(raw) as IterationDragPayload
                  if (payload.iterationNumber === iteration.number) return
                  const targetOrder = positionInIterationOrder(iteration)
                  if (targetOrder === null) return
                  const allOrders = sortedIterations
                    .map((it) => positionInIterationOrder(it))
                    .filter((p): p is number => p !== null)
                  const targetIdx = allOrders.indexOf(targetOrder)
                  if (targetIdx === -1) return
                  const draggedOldOrder = payload.oldOrder
                  if (draggedOldOrder === null) return
                  const mutation = computeDropMutation({
                    draggedNumber: payload.iterationNumber,
                    draggedThemeName: 'order',
                    draggedOldPosition: draggedOldOrder,
                    targetThemeName: 'order',
                    targetMemberPositions: allOrders,
                    targetMemberIndex: targetIdx,
                    targetMembersIncludeDragged: true
                  })
                  if (!mutation) return
                  postLabelMutation(payload.iterationNumber, mutation.add, mutation.remove)
                }}
                onToggle={(event) => {
                  collapse.setOpen(key, event.currentTarget.open)
                }}
              >
                <summary className="board-iteration-summary">
                  {iteration.gh_state !== 'closed' ? (
                    <span
                      className="board-iteration-grip"
                      data-test={ITERATION_GRIP_TEST_ID}
                      draggable={true}
                      role="button"
                      aria-label={`Drag to reorder iteration ${iteration.title}`}
                      tabIndex={-1}
                      onClick={(event) => {
                        event.preventDefault()
                        event.stopPropagation()
                      }}
                      onDragStart={(event) => {
                        const payload: IterationDragPayload = {
                          iterationNumber: iteration.number,
                          oldOrder: positionInIterationOrder(iteration)
                        }
                        event.dataTransfer.setData(
                          DRAG_MIMETYPE_ITERATION,
                          JSON.stringify(payload)
                        )
                        event.dataTransfer.effectAllowed = 'move'
                        const details = (event.currentTarget as HTMLElement).closest('details')
                        if (details && typeof event.dataTransfer.setDragImage === 'function') {
                          const rect = details.getBoundingClientRect()
                          event.dataTransfer.setDragImage(
                            details,
                            event.clientX - rect.left,
                            event.clientY - rect.top
                          )
                        }
                        setDraggedIterationNumber(iteration.number)
                      }}
                      onDragEnd={() => setDraggedIterationNumber(null)}
                    >
                      <GripIcon />
                    </span>
                  ) : null}
                  <button
                    type="button"
                    className="board-iteration-open-button"
                    onClick={(event) => {
                      event.preventDefault()
                      event.stopPropagation()
                      onOpen(iteration.number)
                    }}
                    onKeyDown={(event) => {
                      if (!event.ctrlKey || !event.shiftKey) return
                      if (iteration.gh_state === 'closed') return
                      const direction =
                        event.key === 'ArrowUp' ? 'up' : event.key === 'ArrowDown' ? 'down' : null
                      if (direction === null) return
                      event.preventDefault()
                      const iterationIdx = sortedIterations.findIndex(
                        (it) => it.number === iteration.number
                      )
                      if (iterationIdx === -1) return
                      const myOrder = positionInIterationOrder(iteration)
                      if (myOrder === null) return
                      if (direction === 'up' && iterationIdx === 0) {
                        onReorderAnnounce?.(
                          `${iteration.title} is already at position 1 among iterations`
                        )
                        return
                      }
                      if (direction === 'down' && iterationIdx === sortedIterations.length - 1) {
                        onReorderAnnounce?.(
                          `${iteration.title} is already at last position among iterations`
                        )
                        return
                      }
                      const allOrders = sortedIterations
                        .map((it) => positionInIterationOrder(it))
                        .filter((p): p is number => p !== null)
                      const targetMemberIndex =
                        direction === 'up' ? iterationIdx - 1 : iterationIdx + 1
                      const mutation = computeDropMutation({
                        draggedNumber: iteration.number,
                        draggedThemeName: 'order',
                        draggedOldPosition: myOrder,
                        targetThemeName: 'order',
                        targetMemberPositions: allOrders,
                        targetMemberIndex,
                        targetMembersIncludeDragged: true
                      })
                      if (!mutation) return
                      postLabelMutation(iteration.number, mutation.add, mutation.remove)
                      const newIdx = direction === 'up' ? iterationIdx - 1 : iterationIdx + 1
                      onReorderAnnounce?.(
                        `Moved ${iteration.title} to position ${newIdx + 1} of ${sortedIterations.length} among iterations`
                      )
                    }}
                    title={iteration.title}
                    aria-label={`open iteration #${iteration.number} ${iteration.title}`}
                  >
                    <StoryTypeGlyph type="iteration" />
                    <small className="board-story-number">#{iteration.number}</small>
                    <b className="board-iteration-title">{iteration.title}</b>
                    {iteration.gh_state === 'closed' && (
                      <span className="board-iteration-closed-badge">(closed)</span>
                    )}
                    <small className="board-iteration-count">({members.length})</small>
                  </button>
                </summary>
                <MemberList
                  iteration={iteration}
                  themeName={themeName}
                  members={members}
                  draggedNumber={draggedNumber}
                  setDraggedNumber={setDraggedNumber}
                  hoveredNumber={hoveredNumber}
                  setHoveredNumber={setHoveredNumber}
                  setPending={setPending}
                  onOpen={onOpen}
                  onReorderAnnounce={onReorderAnnounce}
                />
              </details>
            </li>
          )
        })}
        <UnscheduledList
          stories={sortedLoose}
          open={collapse.isOpen(UNSCHEDULED_KEY, true)}
          onToggle={(o) => collapse.setOpen(UNSCHEDULED_KEY, o)}
          onOpen={onOpen}
        />
      </ul>
    </section>
  )
}

export default Iteration
