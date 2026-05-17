import { GripIcon } from './GripIcon'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { computeDropMutation } from './iterationPosition'
import { parseIterationLabel } from './labelSchema'
import { STORY_GRIP_TEST_ID } from './testIds'
import { storyTypeFromLabels, type BoardStory } from './types'
import {
  DRAG_MIMETYPE,
  type DragPayload,
  eagerReorder,
  postLabelMutation
} from './iterationMembers'
import type { IterationMember } from './useIterationMembers'

interface Props {
  iteration: BoardStory
  themeName: string
  members: IterationMember[]
  draggedNumber: number | null
  setDraggedNumber: (n: number | null) => void
  hoveredNumber: number | null
  setHoveredNumber: (n: number | null) => void
  setPending: (storyNumber: number, theme: string, position: number) => void
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const MemberList = ({
  iteration,
  themeName,
  members,
  draggedNumber,
  setDraggedNumber,
  hoveredNumber,
  setHoveredNumber,
  setPending,
  onOpen,
  onReorderAnnounce
}: Props): React.JSX.Element => {
  const orderedMembers = eagerReorder(
    members,
    (m) => m.story.number,
    draggedNumber,
    hoveredNumber
  )
  return (
    <ul className="board-iteration-body">
      {orderedMembers.map(({ story, position }) => (
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
            const parsed = addedLabel ? parseIterationLabel(addedLabel) : null
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
                event.dataTransfer.setData(DRAG_MIMETYPE, JSON.stringify(payload))
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
      ))}
    </ul>
  )
}

export default MemberList
