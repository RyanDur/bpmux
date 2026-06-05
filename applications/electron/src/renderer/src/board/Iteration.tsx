import { useState } from 'react'
import { GripIcon } from './GripIcon'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { useCollapsibleSectionsStorage } from '../lib/useCollapsibleSectionsStorage'
import { computeIterationOrderMutation } from './iterationPosition'
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
import { useDragReorder } from './useDragReorder'
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

const decodeIterationPayload = (raw: string): IterationDragPayload | null => {
  try {
    return JSON.parse(raw) as IterationDragPayload
  } catch {
    return null
  }
}

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<number | null>(
    null
  )
  const { sortedIterations, sortedLoose, getMembersFor, setPending } = useIterationMembers(stories)
  const sectionStorage = useCollapsibleSectionsStorage<IterationSectionId>(
    ITERATION_SECTIONS_STORAGE_PREFIX,
    RENDERER_IDENTITY,
    ITERATION_SECTION_DEFAULTS
  )

  const activePlannedIterations = sortedIterations.filter((it) => it.gh_state !== 'closed')
  const closedIterations = sortedIterations.filter((it) => it.gh_state === 'closed')

  const {
    orderedItems: orderedActiveIterations,
    draggedNumber,
    rowProps,
    gripProps
  } = useDragReorder<IterationDragPayload, BoardStory>({
    items: activePlannedIterations,
    keyOf: (it) => it.number,
    mimeType: DRAG_MIMETYPE_REGISTRY.ITERATION_ITSELF,
    encode: JSON.stringify,
    decode: decodeIterationPayload,
    payloadKey: (p) => p.iterationNumber,
    buildPayload: (it) => ({
      iterationNumber: it.number,
      oldOrder: positionInIterationOrder(it)
    }),
    onReorder: (payload, targetKey) => {
      if (payload.oldOrder === null) return
      const targetIndex = sortedIterations.findIndex((it) => it.number === targetKey)
      if (targetIndex === -1) return
      const allOrders = sortedIterations
        .map((it) => positionInIterationOrder(it))
        .filter((p): p is number => p !== null)
      const mutation = computeIterationOrderMutation({
        draggedOldPosition: payload.oldOrder,
        targetMemberPositions: allOrders,
        targetMemberIndex: targetIndex,
        targetMembersIncludeDragged: true
      })
      if (!mutation) return
      postLabelMutation(payload.iterationNumber, mutation.add, mutation.remove)
    }
  })

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
    const iterationIdx = sortedIterations.findIndex((it) => it.number === iteration.number)
    if (iterationIdx === -1) return
    const myOrder = positionInIterationOrder(iteration)
    if (myOrder === null) return
    if (direction === 'up' && iterationIdx === 0) {
      onReorderAnnounce?.(`${iteration.title} is already at position 1 among iterations`)
      return
    }
    if (direction === 'down' && iterationIdx === sortedIterations.length - 1) {
      onReorderAnnounce?.(`${iteration.title} is already at last position among iterations`)
      return
    }
    const allOrders = sortedIterations
      .map((it) => positionInIterationOrder(it))
      .filter((p): p is number => p !== null)
    const targetMemberIndex = direction === 'up' ? iterationIdx - 1 : iterationIdx + 1
    const mutation = computeIterationOrderMutation({
      draggedOldPosition: myOrder,
      targetMemberPositions: allOrders,
      targetMemberIndex,
      targetMembersIncludeDragged: true
    })
    if (!mutation) return
    postLabelMutation(iteration.number, mutation.add, mutation.remove)
    onReorderAnnounce?.(
      `Moved ${iteration.title} to position ${targetMemberIndex + 1} of ${sortedIterations.length} among iterations`
    )
  }

  const renderIterationRow = (iteration: BoardStory, draggable: boolean): React.JSX.Element => {
    const key = String(iteration.number)
    const themeName = themeNameFromTitle(iteration.title)
    const members = getMembersFor(iteration.number)
    const defaultOpen = iteration.gh_state !== 'closed'
    const open = collapse.isOpen(key, defaultOpen)
    const isCurrent = iteration.number === currentIterationNumber
    return (
      <li key={iteration.number} className="board-iteration-item">
        <article className="board-iteration-row" {...(draggable ? rowProps(iteration) : {})}>
          <details
            className="board-iteration-card"
            data-test={boardIterationTestId(iteration.number)}
            data-iteration-kind="named"
            data-iteration-name={themeName}
            data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
            data-current-iteration={isCurrent ? 'true' : undefined}
            data-dragging={draggable && draggedNumber === iteration.number ? 'true' : undefined}
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
          {draggable ? (
            <button
              type="button"
              className="board-iteration-grip"
              data-test={ITERATION_GRIP_TEST_ID}
              tabIndex={0}
              aria-label={`Reorder iteration #${iteration.number} ${iteration.title} — drag, or Ctrl+Shift+Up/Down`}
              aria-keyshortcuts="Control+Shift+ArrowUp Control+Shift+ArrowDown"
              {...gripProps(iteration)}
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

  const activePlannedCount = orderedActiveIterations.length
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
        <ul className="board-iterations-list">
          {orderedActiveIterations.map((it) => renderIterationRow(it, true))}
        </ul>
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
          {closedIterations.map((it) => renderIterationRow(it, false))}
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
