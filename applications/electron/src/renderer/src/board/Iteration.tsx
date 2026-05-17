import { useState } from 'react'
import { GripIcon } from './GripIcon'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { computeDropMutation } from './iterationPosition'
import { parseIterationLabel, themeNameFromTitle } from './labelSchema'
import {
  ITERATION_GRIP_TEST_ID,
  STORY_GRIP_TEST_ID,
  boardIterationTestId
} from './testIds'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { storyTypeFromLabels, type BoardStory } from './types'
import {
  DRAG_MIMETYPE,
  DRAG_MIMETYPE_ITERATION,
  type DragPayload,
  type IterationDragPayload,
  eagerReorder,
  positionInIterationOrder,
  postLabelMutation
} from './iterationMembers'
import { useIterationMembers } from './useIterationMembers'

interface Props {
  stories: BoardStory[]
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const UNSCHEDULED_KEY = 'unscheduled'

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const [draggedNumber, setDraggedNumber] = useState<number | null>(null)
  const [hoveredNumber, setHoveredNumber] = useState<number | null>(null)
  const [draggedIterationNumber, setDraggedIterationNumber] = useState<number | null>(null)
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<number | null>(
    null
  )
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
                <ul className="board-iteration-body">
                  {eagerReorder(members, (m) => m.story.number, draggedNumber, hoveredNumber).map(
                    ({ story, position }) => (
                      <li
                        key={story.number}
                        className="board-iteration-member"
                        data-test={`board-story-${story.number}`}
                        data-iteration-member={story.number}
                        data-dragging={draggedNumber === story.number ? 'true' : undefined}
                        onDragOver={(event) => {
                          if (event.dataTransfer.types.includes(DRAG_MIMETYPE)) {
                            event.preventDefault()
                            event.dataTransfer.dropEffect = 'move'
                            if (draggedNumber !== null && draggedNumber !== story.number) {
                              setHoveredNumber(story.number)
                            }
                          }
                        }}
                        onDrop={(event) => {
                          const raw = event.dataTransfer.getData(DRAG_MIMETYPE)
                          if (!raw) return
                          event.preventDefault()
                          const payload = JSON.parse(raw) as DragPayload
                          if (iteration.gh_state === 'closed') return
                          const targetNumber =
                            payload.storyNumber === story.number && hoveredNumber !== null
                              ? hoveredNumber
                              : story.number
                          if (payload.storyNumber === targetNumber) return
                          const originalIdx = members.findIndex(
                            (m) => m.story.number === targetNumber
                          )
                          if (originalIdx === -1) return
                          const mutation = computeDropMutation({
                            draggedNumber: payload.storyNumber,
                            draggedThemeName: payload.themeName,
                            draggedOldPosition: payload.oldPosition,
                            targetThemeName: themeName,
                            targetMemberPositions: members.map((m) => m.position),
                            targetMemberIndex: originalIdx,
                            targetMembersIncludeDragged: payload.themeName === themeName
                          })
                          if (!mutation) return
                          postLabelMutation(payload.storyNumber, mutation.add, mutation.remove)
                          const addedLabel = mutation.add[0]
                          const parsed = addedLabel
                            ? parseIterationLabel(addedLabel)
                            : null
                          if (parsed) {
                            setPending(payload.storyNumber, parsed.name, parsed.position)
                          }
                          setDraggedNumber(null)
                          setHoveredNumber(null)
                        }}
                      >
                        {iteration.gh_state !== 'closed' ? (
                          <span
                            className="board-story-grip"
                            data-test={STORY_GRIP_TEST_ID}
                            draggable={true}
                            role="button"
                            aria-label={`Drag to reorder #${story.number}`}
                            tabIndex={-1}
                            onDragStart={(event) => {
                              const payload: DragPayload = {
                                storyNumber: story.number,
                                themeName,
                                oldPosition: position
                              }
                              event.dataTransfer.setData(
                                DRAG_MIMETYPE,
                                JSON.stringify(payload)
                              )
                              event.dataTransfer.effectAllowed = 'move'
                              const row = (event.currentTarget as HTMLElement).closest('li')
                              if (row && typeof event.dataTransfer.setDragImage === 'function') {
                                const rect = row.getBoundingClientRect()
                                event.dataTransfer.setDragImage(
                                  row,
                                  event.clientX - rect.left,
                                  event.clientY - rect.top
                                )
                              }
                              setDraggedNumber(story.number)
                            }}
                            onDragEnd={() => {
                              setDraggedNumber(null)
                              setHoveredNumber(null)
                            }}
                          >
                            <GripIcon />
                          </span>
                        ) : null}
                        <div
                          role="button"
                          tabIndex={0}
                          className="board-iteration-member-button board-story-button"
                          onClick={() => onOpen(story.number)}
                          onKeyDown={(event) => {
                            if (event.key === 'Enter' || event.key === ' ') {
                              event.preventDefault()
                              onOpen(story.number)
                              return
                            }
                            if (!event.ctrlKey || !event.shiftKey) return
                            if (iteration.gh_state === 'closed') return
                            const direction =
                              event.key === 'ArrowUp'
                                ? 'up'
                                : event.key === 'ArrowDown'
                                  ? 'down'
                                  : null
                            if (direction === null) return
                            event.preventDefault()
                            const originalIdx = members.findIndex(
                              (m) => m.story.number === story.number
                            )
                            if (originalIdx === -1) return
                            if (direction === 'up' && originalIdx === 0) {
                              onReorderAnnounce?.(
                                `${story.title} is already at position 1 in ${iteration.title}`
                              )
                              return
                            }
                            if (direction === 'down' && originalIdx === members.length - 1) {
                              onReorderAnnounce?.(
                                `${story.title} is already at last position in ${iteration.title}`
                              )
                              return
                            }
                            const targetMemberIndex =
                              direction === 'up' ? originalIdx - 1 : originalIdx + 1
                            const mutation = computeDropMutation({
                              draggedNumber: story.number,
                              draggedThemeName: themeName,
                              draggedOldPosition: position,
                              targetThemeName: themeName,
                              targetMemberPositions: members.map((m) => m.position),
                              targetMemberIndex,
                              targetMembersIncludeDragged: true
                            })
                            if (!mutation) return
                            postLabelMutation(story.number, mutation.add, mutation.remove)
                            const addedLabel = mutation.add[0]
                            const parsedKey = addedLabel
                              ? parseIterationLabel(addedLabel)
                              : null
                            if (parsedKey) {
                              setPending(story.number, parsedKey.name, parsedKey.position)
                            }
                            const newIdx = direction === 'up' ? originalIdx - 1 : originalIdx + 1
                            onReorderAnnounce?.(
                              `Moved ${story.title} to position ${newIdx + 1} of ${members.length} in ${iteration.title}`
                            )
                          }}
                        >
                          <small className="board-story-number">#{story.number}</small>
                          <StoryTypeGlyph type={storyTypeFromLabels(story.labels)} />
                          <StoryBadge story={story} onOpenBlocker={onOpen} />
                          <b className="board-story-title" data-test="story-title">
                            {story.title}
                          </b>
                          <ReworkGlyph reworkComment={story.rework_comment} />
                          {typeof story.parent_epic === 'number' ? (
                            <button
                              type="button"
                              className="board-story-epic-ref"
                              data-test="epic-link"
                              aria-label={`open epic #${story.parent_epic}`}
                              onClick={(event) => {
                                event.preventDefault()
                                event.stopPropagation()
                                onOpen(story.parent_epic as number)
                              }}
                            >
                              ↳ Epic #{story.parent_epic}
                            </button>
                          ) : null}
                        </div>
                      </li>
                    )
                  )}
                </ul>
              </details>
            </li>
          )
        })}
        {sortedLoose.length > 0 && (
          <li key="unscheduled" className="board-iteration-item">
            <details
              className="board-iteration-card board-iteration-card--unscheduled"
              data-test="board-iteration-unscheduled"
              data-iteration-kind="unscheduled"
              open={collapse.isOpen(UNSCHEDULED_KEY, true)}
              onToggle={(event) => {
                collapse.setOpen(UNSCHEDULED_KEY, event.currentTarget.open)
              }}
            >
              <summary className="board-iteration-summary">
                <b className="board-iteration-title">Unscheduled</b>
                <small className="board-iteration-count">({sortedLoose.length})</small>
              </summary>
              <ul className="board-iteration-body">
                {sortedLoose.map((story) => (
                  <li
                    key={story.number}
                    className="board-iteration-member"
                    data-test={`board-story-${story.number}`}
                    data-iteration-member={story.number}
                  >
                    <div
                      role="button"
                      tabIndex={0}
                      className="board-iteration-member-button board-story-button"
                      onClick={() => onOpen(story.number)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault()
                          onOpen(story.number)
                        }
                      }}
                    >
                      <small className="board-story-number">#{story.number}</small>
                      <StoryTypeGlyph type={storyTypeFromLabels(story.labels)} />
                      <StoryBadge story={story} onOpenBlocker={onOpen} />
                      <b className="board-story-title" data-test="story-title">
                        {story.title}
                      </b>
                      <ReworkGlyph reworkComment={story.rework_comment} />
                      {typeof story.parent_epic === 'number' ? (
                        <button
                          type="button"
                          className="board-story-epic-ref"
                          data-test="epic-link"
                          aria-label={`open epic #${story.parent_epic}`}
                          onClick={(event) => {
                            event.preventDefault()
                            event.stopPropagation()
                            onOpen(story.parent_epic as number)
                          }}
                        >
                          ↳ Epic #{story.parent_epic}
                        </button>
                      ) : null}
                    </div>
                  </li>
                ))}
              </ul>
            </details>
          </li>
        )}
      </ul>
    </section>
  )
}

export default Iteration
