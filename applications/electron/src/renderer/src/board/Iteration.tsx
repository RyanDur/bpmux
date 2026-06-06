import { useState } from 'react'
import { GripIcon } from './GripIcon'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { useCollapsibleSectionsStorage } from '../lib/useCollapsibleSectionsStorage'
import {
  computeEstablishOrderMutations,
  computeIterationOrderMutation,
  type DropMutation
} from './iterationPosition'
import { parseIterationOrderPosition, themeNameFromTitle } from './labelSchema'
import { ITERATION_GRIP_TEST_ID, boardIterationTestId } from './testIds'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { findCurrentIteration } from './currentIteration'
import type { BoardStory } from './types'
import {
  type IterationDragPayload,
  eagerReorder,
  positionInIterationOrder,
  postLabelMutation
} from './iterationMembers'
import { DRAG_MIMETYPE_REGISTRY } from './useStoryDrag'
import { SortableList, type SortableRowContext } from './SortableList'
import { ITERATION_ITSELF_DRAG } from './sortableDragConfigs'
import { useIterationMembers } from './useIterationMembers'
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
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<number | null>(
    null
  )
  const { sortedIterations, sortedLoose, getMembersFor, setPending, setPendingIterationOrder } =
    useIterationMembers(stories)
  const sectionStorage = useCollapsibleSectionsStorage<IterationSectionId>(
    ITERATION_SECTIONS_STORAGE_PREFIX,
    RENDERER_IDENTITY,
    ITERATION_SECTION_DEFAULTS
  )

  const activePlannedIterations = sortedIterations.filter((it) => it.gh_state !== 'closed')
  const closedIterations = sortedIterations.filter((it) => it.gh_state === 'closed')

  const applyOrderMutation = (iterationNumber: number, mutation: DropMutation): void => {
    postLabelMutation(iterationNumber, mutation.add, mutation.remove)
    const newOrder = parseIterationOrderPosition(mutation.add[0])
    if (newOrder !== null) {
      setPendingIterationOrder(iterationNumber, newOrder)
    }
  }

  const placeIteration = (movedNumber: number, targetMemberIndex: number): number | null => {
    const moved = activePlannedIterations.find((it) => it.number === movedNumber)
    if (moved === undefined) return null
    const targetNumber = activePlannedIterations[targetMemberIndex]?.number ?? null
    const newSequence = eagerReorder(
      activePlannedIterations,
      (it) => it.number,
      movedNumber,
      targetNumber
    )
    const finalIndex = newSequence.findIndex((it) => it.number === movedNumber)
    if (finalIndex === -1) return null

    const draggedOldPosition = positionInIterationOrder(moved)
    const fullyOrdered = activePlannedIterations.every(
      (it) => positionInIterationOrder(it) !== null
    )

    if (fullyOrdered && draggedOldPosition !== null) {
      const activeOrders = activePlannedIterations
        .map((it) => positionInIterationOrder(it))
        .filter((p): p is number => p !== null)
      const mutation = computeIterationOrderMutation({
        draggedOldPosition,
        targetMemberPositions: activeOrders,
        targetMemberIndex,
        targetMembersIncludeDragged: true
      })
      if (mutation === null) return null
      applyOrderMutation(movedNumber, mutation)
      return finalIndex + 1
    }

    const mutations = computeEstablishOrderMutations(
      newSequence.map((it) => ({
        number: it.number,
        currentPosition: positionInIterationOrder(it)
      }))
    )
    if (mutations.length === 0) return null
    for (const m of mutations) {
      applyOrderMutation(m.iterationNumber, m)
    }
    return finalIndex + 1
  }

  const reorderIterationByDrag = (payload: IterationDragPayload, targetKey: number): void => {
    const targetIndex = activePlannedIterations.findIndex((it) => it.number === targetKey)
    if (targetIndex === -1) return
    placeIteration(payload.iterationNumber, targetIndex)
  }

  const sectionOpen = sectionStorage.state
  const toggleSection = (id: IterationSectionId, next: boolean): void => {
    if (sectionOpen[id] === next) return
    sectionStorage.setState({ ...sectionOpen, [id]: next })
  }
  if (sortedIterations.length === 0 && sortedLoose.length === 0) return null
  const currentIterationNumber = findCurrentIteration(stories)?.number ?? null

  const rejectClosedDrop =
    (iteration: BoardStory) =>
    (event: React.DragEvent<HTMLElement>): void => {
      if (
        event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER) ||
        event.dataTransfer.types.includes(DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF)
      ) {
        event.dataTransfer.dropEffect = 'none'
        setDragRejectedIterationNumber(iteration.number)
      }
    }

  const reorderIterationByKeyboard = (iteration: BoardStory, direction: 'up' | 'down'): void => {
    const iterationIdx = activePlannedIterations.findIndex((it) => it.number === iteration.number)
    if (iterationIdx === -1) return
    if (direction === 'up' && iterationIdx === 0) {
      onReorderAnnounce?.(`${iteration.title} is already at position 1 among iterations`)
      return
    }
    if (direction === 'down' && iterationIdx === activePlannedIterations.length - 1) {
      onReorderAnnounce?.(`${iteration.title} is already at last position among iterations`)
      return
    }
    const targetMemberIndex = direction === 'up' ? iterationIdx - 1 : iterationIdx + 1
    const newPosition = placeIteration(iteration.number, targetMemberIndex)
    if (newPosition === null) return
    onReorderAnnounce?.(
      `Moved ${iteration.title} to position ${newPosition} of ${activePlannedIterations.length} among iterations`
    )
  }

  const renderIterationRow = (
    iteration: BoardStory,
    drag?: SortableRowContext
  ): React.JSX.Element => {
    const key = String(iteration.number)
    const themeName = themeNameFromTitle(iteration.title)
    const members = getMembersFor(iteration.number)
    const defaultOpen = iteration.gh_state !== 'closed'
    const open = collapse.isOpen(key, defaultOpen)
    const isCurrent = iteration.number === currentIterationNumber
    const draggable = drag !== undefined
    return (
      <li key={iteration.number} className="board-iteration-item">
        <article className="board-iteration-row" {...(drag ? drag.rowDragProps : {})}>
          <details
            className="board-iteration-card"
            data-test={boardIterationTestId(iteration.number)}
            data-iteration-kind="named"
            data-iteration-name={themeName}
            data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
            data-current-iteration={isCurrent ? 'true' : undefined}
            data-dragging={drag?.dragging ? 'true' : undefined}
            data-drag-rejected={
              dragRejectedIterationNumber === iteration.number ? 'true' : undefined
            }
            open={open}
            onDragOver={draggable ? undefined : rejectClosedDrop(iteration)}
            onDragLeave={
              draggable
                ? undefined
                : () => {
                    if (dragRejectedIterationNumber === iteration.number) {
                      setDragRejectedIterationNumber(null)
                    }
                  }
            }
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
              {iteration.gh_state === 'closed' && (
                <small className="board-iteration-closed-badge">(closed)</small>
              )}
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
                reorderIterationByKeyboard(iteration, direction)
              }}
            >
              <GripIcon />
            </button>
          ) : null}
        </article>
      </li>
    )
  }

  const activePlannedCount = activePlannedIterations.length
  return (
    <section className="board-iterations" data-test="board-iterations">
      <details
        className="board-iterations-section board-iterations-section--active"
        data-test="board-iterations-section-active-planned"
        open={sectionOpen['active-planned']}
        onToggle={(event) => toggleSection('active-planned', event.currentTarget.open)}
      >
        <summary
          className="board-iterations-section-summary disclosure-summary"
          data-test="board-iterations-section-active-planned-summary"
        >
          <i className="board-iterations-section-chevron" aria-hidden="true" />
          <b className="board-iterations-section-label">Active + Planned</b>
          <small
            className="board-iterations-section-count"
            data-test="board-iterations-section-active-planned-count"
          >
            ({activePlannedCount})
          </small>
        </summary>
        <SortableList<IterationDragPayload, BoardStory>
          items={activePlannedIterations}
          keyOf={(it) => it.number}
          rowKey={(it) => it.number}
          config={ITERATION_ITSELF_DRAG}
          buildPayload={(it) => ({
            iterationNumber: it.number
          })}
          onReorder={reorderIterationByDrag}
          ulClassName="board-iterations-list"
          renderRow={(it, ctx) => renderIterationRow(it, ctx)}
        />
      </details>
      <details
        className="board-iterations-section board-iterations-section--closed"
        data-test="board-iterations-section-closed"
        open={sectionOpen.closed}
        onToggle={(event) => toggleSection('closed', event.currentTarget.open)}
      >
        <summary
          className="board-iterations-section-summary disclosure-summary"
          data-test="board-iterations-section-closed-summary"
        >
          <i className="board-iterations-section-chevron" aria-hidden="true" />
          <b className="board-iterations-section-label">Closed</b>
          <small
            className="board-iterations-section-count"
            data-test="board-iterations-section-closed-count"
          >
            ({closedIterations.length})
          </small>
        </summary>
        <ul className="board-iterations-list">
          {closedIterations.map((it) => renderIterationRow(it))}
        </ul>
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
