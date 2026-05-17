import { useEffect, useState } from 'react'
import { GripIcon } from './GripIcon'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { computeDropMutation } from './iterationPosition'
import {
  parseIterationLabel,
  parseIterationOrderPosition,
  parseUnscheduledPosition,
  themeNameFromTitle
} from './labelSchema'
import {
  ITERATION_GRIP_TEST_ID,
  STORY_GRIP_TEST_ID,
  boardIterationTestId
} from './testIds'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { postStoryLabels } from '../api/labelsClient'
import { storyTypeFromLabels, type BoardStory } from './types'

interface Props {
  stories: BoardStory[]
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const UNSCHEDULED_KEY = 'unscheduled'

const positionInIterationOrder = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    const n = parseIterationOrderPosition(label)
    if (n !== null) return n
  }
  return null
}

const isIterationStory = (story: BoardStory): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

const positionInIteration = (story: BoardStory, themeName: string): number | null => {
  for (const label of story.labels) {
    const parsed = parseIterationLabel(label)
    if (parsed?.name === themeName) return parsed.position
  }
  return null
}

const positionInUnscheduled = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    const n = parseUnscheduledPosition(label)
    if (n !== null) return n
  }
  return null
}

const isLooseStory = (story: BoardStory): boolean => {
  if (isIterationStory(story)) return false
  if (story.state !== 'icebox') return false
  const hasNamedIterationLabel = story.labels.some((l) => {
    const parsed = parseIterationLabel(l)
    return parsed !== null && parsed.name !== 'Unscheduled'
  })
  return !hasNamedIterationLabel
}

interface DragPayload {
  storyNumber: number
  themeName: string
  oldPosition: number
}

interface IterationDragPayload {
  iterationNumber: number
  oldOrder: number | null
}

const DRAG_MIMETYPE = 'application/x-bpmux-iteration-member'
const DRAG_MIMETYPE_ITERATION = 'application/x-bpmux-iteration-itself'

const postLabelMutation = (
  storyNumber: number,
  addLabels: string[],
  removeLabels: string[]
): void => {
  if (typeof window === 'undefined') return
  const bridge = window.bpmux
  if (!bridge?.apiBase) return
  void postStoryLabels(
    { apiBase: bridge.apiBase, authToken: bridge.authToken },
    storyNumber,
    { add: addLabels, remove: removeLabels }
  ).value
}

const eagerReorder = <T,>(
  items: T[],
  findId: (t: T) => number,
  fromId: number | null,
  toId: number | null
): T[] => {
  if (fromId === null || toId === null) return items
  const fromIdx = items.findIndex((t) => findId(t) === fromId)
  const toIdx = items.findIndex((t) => findId(t) === toId)
  if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return items
  const next = [...items]
  const [moved] = next.splice(fromIdx, 1)
  next.splice(toIdx, 0, moved)
  return next
}

interface PendingMemberPosition {
  theme: string
  position: number
}

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const [draggedNumber, setDraggedNumber] = useState<number | null>(null)
  const [hoveredNumber, setHoveredNumber] = useState<number | null>(null)
  const [draggedIterationNumber, setDraggedIterationNumber] = useState<number | null>(null)
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<number | null>(
    null
  )
  const [pendingMemberPosition, setPendingMemberPosition] = useState<
    Record<number, PendingMemberPosition>
  >({})

  useEffect(() => {
    setPendingMemberPosition((prev) => {
      const next: Record<number, PendingMemberPosition> = {}
      let changed = false
      for (const [numStr, pending] of Object.entries(prev)) {
        const num = Number(numStr)
        const story = stories.find((s) => s.number === num)
        const actualPosition = story ? positionInIteration(story, pending.theme) : null
        if (actualPosition === pending.position) {
          changed = true
          continue
        }
        next[num] = pending
      }
      return changed ? next : prev
    })
  }, [stories])
  const iterations = [...stories.filter(isIterationStory)].sort((a, b) => {
    const ao = positionInIterationOrder(a)
    const bo = positionInIterationOrder(b)
    if (ao !== null && bo !== null) return ao - bo
    if (ao !== null) return -1
    if (bo !== null) return 1
    return a.number - b.number
  })
  const loose = stories.filter(isLooseStory)
  if (iterations.length === 0 && loose.length === 0) return null

  const sortedLoose = [...loose].sort((a, b) => {
    const ap = positionInUnscheduled(a)
    const bp = positionInUnscheduled(b)
    if (ap !== null && bp !== null) return ap - bp
    if (ap !== null) return -1
    if (bp !== null) return 1
    return a.number - b.number
  })

  return (
    <section className="board-iterations" data-test="board-iterations">
      <ul className="board-iterations-list">
        {iterations.map((iteration) => {
          const key = String(iteration.number)
          const themeName = themeNameFromTitle(iteration.title)
          const members = stories
            .map((s) => {
              if (isIterationStory(s)) return null
              const pending = pendingMemberPosition[s.number]
              const labelPosition = positionInIteration(s, themeName)
              const pendingPosition =
                pending !== undefined && pending.theme === themeName ? pending.position : null
              if (pendingPosition !== null) return { story: s, position: pendingPosition }
              if (labelPosition === null) return null
              if (
                pending !== undefined &&
                pending.theme !== themeName &&
                pending.theme.toLowerCase() !== themeName.toLowerCase()
              ) {
                return null
              }
              return { story: s, position: labelPosition }
            })
            .filter((m): m is { story: BoardStory; position: number } => m !== null)
            .sort((a, b) => a.position - b.position)
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
                  const allOrders = iterations
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
                      const iterationIdx = iterations.findIndex(
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
                      if (direction === 'down' && iterationIdx === iterations.length - 1) {
                        onReorderAnnounce?.(
                          `${iteration.title} is already at last position among iterations`
                        )
                        return
                      }
                      const allOrders = iterations
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
                        `Moved ${iteration.title} to position ${newIdx + 1} of ${iterations.length} among iterations`
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
                            setPendingMemberPosition((prev) => ({
                              ...prev,
                              [payload.storyNumber]: {
                                theme: parsed.name,
                                position: parsed.position
                              }
                            }))
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
                              setPendingMemberPosition((prev) => ({
                                ...prev,
                                [story.number]: {
                                  theme: parsedKey.name,
                                  position: parsedKey.position
                                }
                              }))
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
