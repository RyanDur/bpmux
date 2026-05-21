import { GripIcon } from './GripIcon'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { useCollapsibleSectionsStorage } from '../lib/useCollapsibleSectionsStorage'
import { computeDropMutation } from './iterationPosition'
import { themeNameFromTitle } from './labelSchema'
import { ITERATION_GRIP_TEST_ID, boardIterationTestId } from './testIds'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { findCurrentIteration } from './currentIteration'
import type { BoardStory } from './types'
import {
  type IterationDragPayload,
  positionInIterationOrder,
  postLabelMutation
} from './iterationMembers'
import { DRAG_MIMETYPE_REGISTRY } from './useStoryDrag'
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

type IterationSectionId = 'active-planned' | 'closed'

const ITERATION_SECTIONS_STORAGE_PREFIX = 'bpmux-iteration-section-open'
const ITERATION_SECTION_DEFAULTS: Record<IterationSectionId, boolean> = {
  'active-planned': true,
  closed: false
}

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const {
    draggedIterationNumber,
    setDraggedIterationNumber,
    dragRejectedIterationNumber,
    setDragRejectedIterationNumber
  } = useIterationDragState()
  const { sortedIterations, sortedLoose, getMembersFor, setPending } = useIterationMembers(stories)
  const sectionStorage = useCollapsibleSectionsStorage<IterationSectionId>(
    ITERATION_SECTIONS_STORAGE_PREFIX,
    RENDERER_IDENTITY,
    ITERATION_SECTION_DEFAULTS
  )
  const sectionOpen = sectionStorage.state
  const toggleSection = (id: IterationSectionId, next: boolean): void => {
    if (sectionOpen[id] === next) return
    sectionStorage.setState({ ...sectionOpen, [id]: next })
  }
  if (sortedIterations.length === 0 && sortedLoose.length === 0) return null
  const activePlannedIterations = sortedIterations.filter((it) => it.gh_state !== 'closed')
  const closedIterations = sortedIterations.filter((it) => it.gh_state === 'closed')
  const currentIterationNumber = findCurrentIteration(stories)?.number ?? null

  const renderIterationRow = (iteration: BoardStory): React.JSX.Element => {
    const key = String(iteration.number)
    const themeName = themeNameFromTitle(iteration.title)
    const members = getMembersFor(themeName, iteration.number)
    const defaultOpen = iteration.gh_state !== 'closed'
    const open = collapse.isOpen(key, defaultOpen)
    const isCurrent = iteration.number === currentIterationNumber
    return (
      <li key={iteration.number} className="board-iteration-item">
        <details
          className="board-iteration-card"
          data-test={boardIterationTestId(iteration.number)}
          data-iteration-kind="named"
          data-iteration-name={themeName}
          data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
          data-current-iteration={isCurrent ? 'true' : undefined}
          data-dragging={draggedIterationNumber === iteration.number ? 'true' : undefined}
          data-drag-rejected={dragRejectedIterationNumber === iteration.number ? 'true' : undefined}
          open={open}
          onDragOver={(event) => {
            if (iteration.gh_state === 'closed') {
              if (
                event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER) ||
                event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF)
              ) {
                event.dataTransfer.dropEffect = 'none'
                setDragRejectedIterationNumber(iteration.number)
              }
              return
            }
            if (event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF)) {
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
            const raw = event.dataTransfer.getData(DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF)
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
          <summary className="board-iteration-summary" data-test="board-iteration-summary">
            {iteration.gh_state !== 'closed' ? (
              <button
                type="button"
                className="board-iteration-grip"
                data-test={ITERATION_GRIP_TEST_ID}
                draggable={true}
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
                    DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF,
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
              </button>
            ) : null}
            <button
              type="button"
              className="board-iteration-open-button"
              data-test="board-iteration-open-button"
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
                const targetMemberIndex = direction === 'up' ? iterationIdx - 1 : iterationIdx + 1
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
            setPending={setPending}
            onOpen={onOpen}
            onReorderAnnounce={onReorderAnnounce}
          />
        </details>
      </li>
    )
  }

  return (
    <section className="board-iterations" data-test="board-iterations">
      <details
        className="board-iterations-section board-iterations-section--active"
        data-test="board-iterations-section-active-planned"
        open={sectionOpen['active-planned']}
        onToggle={(event) => toggleSection('active-planned', event.currentTarget.open)}
      >
        <summary
          className="board-iterations-section-summary"
          data-test="board-iterations-section-active-planned-summary"
        >
          <span className="board-iterations-section-chevron" aria-hidden="true" />
          <b className="board-iterations-section-label">Active + Planned</b>
          <small
            className="board-iterations-section-count"
            data-test="board-iterations-section-active-planned-count"
          >
            ({activePlannedIterations.length})
          </small>
        </summary>
        <ul className="board-iterations-list">{activePlannedIterations.map(renderIterationRow)}</ul>
      </details>
      <details
        className="board-iterations-section board-iterations-section--closed"
        data-test="board-iterations-section-closed"
        open={sectionOpen.closed}
        onToggle={(event) => toggleSection('closed', event.currentTarget.open)}
      >
        <summary
          className="board-iterations-section-summary"
          data-test="board-iterations-section-closed-summary"
        >
          <span className="board-iterations-section-chevron" aria-hidden="true" />
          <b className="board-iterations-section-label">Closed</b>
          <small
            className="board-iterations-section-count"
            data-test="board-iterations-section-closed-count"
          >
            ({closedIterations.length})
          </small>
        </summary>
        <ul className="board-iterations-list">{closedIterations.map(renderIterationRow)}</ul>
      </details>
      <UnscheduledList
        stories={sortedLoose}
        open={collapse.isOpen(UNSCHEDULED_KEY, true)}
        onToggle={(o) => collapse.setOpen(UNSCHEDULED_KEY, o)}
        onOpen={onOpen}
      />
    </section>
  )
}

export default Iteration
