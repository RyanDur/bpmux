import { useState } from 'react'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { computeDropMutation } from './iterationPosition'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { storyTypeFromLabels, type BoardStory } from './types'

interface Props {
  stories: BoardStory[]
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const UNSCHEDULED_KEY = 'unscheduled'
const UNSCHEDULED_LABEL_PREFIX = 'iteration-Unscheduled.'
const ITERATION_ORDER_PREFIX = 'iteration-order.'

const positionInIterationOrder = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    if (label.startsWith(ITERATION_ORDER_PREFIX)) {
      const tail = label.slice(ITERATION_ORDER_PREFIX.length)
      const n = parseInt(tail, 10)
      if (!Number.isNaN(n)) return n
    }
  }
  return null
}

const isIterationStory = (story: BoardStory): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

const themeNameFromTitle = (title: string): string => {
  const match = title.match(/^Iteration (.+)$/)
  const raw = match ? match[1].trim() : title.trim()
  return raw.replace(/\s+/g, '-')
}

interface IterationLabelParts {
  name: string
  position: number
}

const parseIterationLabel = (label: string): IterationLabelParts | null => {
  const match = label.match(/^iteration-(.+)\.(\d+)$/)
  if (!match) return null
  return { name: match[1], position: parseInt(match[2], 10) }
}

const positionInIteration = (story: BoardStory, themeName: string): number | null => {
  for (const label of story.labels) {
    const parsed = parseIterationLabel(label)
    if (parsed?.name === themeName) return parsed.position
  }
  return null
}

const positionInUnscheduled = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    if (label.startsWith(UNSCHEDULED_LABEL_PREFIX)) {
      const tail = label.slice(UNSCHEDULED_LABEL_PREFIX.length)
      const n = parseInt(tail, 10)
      if (!Number.isNaN(n)) return n
    }
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
  void fetch(`${bridge.apiBase}/api/stories/${storyNumber}/labels`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${bridge.authToken ?? ''}`
    },
    body: JSON.stringify({ add: addLabels, remove: removeLabels })
  })
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

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const [draggedNumber, setDraggedNumber] = useState<number | null>(null)
  const [hoveredNumber, setHoveredNumber] = useState<number | null>(null)
  const [draggedIterationNumber, setDraggedIterationNumber] = useState<number | null>(null)
  const [dragRejectedIterationNumber, setDragRejectedIterationNumber] = useState<number | null>(
    null
  )
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
              const position = positionInIteration(s, themeName)
              if (position === null) return null
              return { story: s, position }
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
                data-test={`board-iteration-${iteration.number}`}
                data-iteration-kind="named"
                data-iteration-name={themeName}
                data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
                data-dragging={draggedIterationNumber === iteration.number ? 'true' : undefined}
                data-drag-rejected={
                  dragRejectedIterationNumber === iteration.number ? 'true' : undefined
                }
                open={open}
                draggable={iteration.gh_state !== 'closed'}
                onDragStart={(event) => {
                  const payload: IterationDragPayload = {
                    iterationNumber: iteration.number,
                    oldOrder: positionInIterationOrder(iteration)
                  }
                  event.dataTransfer.setData(DRAG_MIMETYPE_ITERATION, JSON.stringify(payload))
                  event.dataTransfer.effectAllowed = 'move'
                  setDraggedIterationNumber(iteration.number)
                }}
                onDragEnd={() => setDraggedIterationNumber(null)}
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
                        direction === 'up' ? iterationIdx - 1 : iterationIdx + 2
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
                        draggable={iteration.gh_state !== 'closed'}
                        data-dragging={draggedNumber === story.number ? 'true' : undefined}
                        onDragStart={(event) => {
                          const payload: DragPayload = {
                            storyNumber: story.number,
                            themeName,
                            oldPosition: position
                          }
                          event.dataTransfer.setData(DRAG_MIMETYPE, JSON.stringify(payload))
                          event.dataTransfer.effectAllowed = 'move'
                          setDraggedNumber(story.number)
                        }}
                        onDragEnd={() => {
                          setDraggedNumber(null)
                          setHoveredNumber(null)
                        }}
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
                          if (payload.storyNumber === story.number) return
                          if (iteration.gh_state === 'closed') return
                          const originalIdx = members.findIndex(
                            (m) => m.story.number === story.number
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
                        }}
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
                              direction === 'up' ? originalIdx - 1 : originalIdx + 2
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
