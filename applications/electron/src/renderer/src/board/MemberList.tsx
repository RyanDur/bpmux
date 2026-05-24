import { GripIcon } from './GripIcon'
import { ReworkGlyph } from './ReworkGlyph'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { computeMemberDropMutation } from './iterationPosition'
import { parseParentIterationLabel } from './labelSchema'
import { STORY_GRIP_TEST_ID } from './testIds'
import { GROUP_ORDER, columnFor, storyTypeFromLabels, type BoardStory } from './types'
import { type DragPayload, eagerReorder, postLabelMutation } from './iterationMembers'
import type { IterationMember } from './useIterationMembers'
import { DRAG_MIMETYPE_REGISTRY, useStoryDrag } from './useStoryDrag'

const decodeDragPayload = (raw: string): DragPayload | null => {
  try {
    return JSON.parse(raw) as DragPayload
  } catch {
    return null
  }
}

const activeStateLabel = (story: BoardStory): string | null => {
  if (story.state === 'icebox') return null
  const column = columnFor(
    story.state,
    story.gh_state === 'closed',
    storyTypeFromLabels(story.labels)
  )
  if (column === 'icebox') return null
  return GROUP_ORDER.find((group) => group.state === column)?.label ?? null
}

interface Props {
  iteration: BoardStory
  members: IterationMember[]
  setPending: (storyNumber: number, iterationNumber: number, position: number) => void
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const MemberList = ({
  iteration,
  members,
  setPending,
  onOpen,
  onReorderAnnounce
}: Props): React.JSX.Element => {
  const { draggedNumber, hoveredNumber, startDrag, endDrag, hover, decode, hasType } =
    useStoryDrag<DragPayload>({
      mimeType: DRAG_MIMETYPE_REGISTRY.ITERATION_MEMBER,
      encode: JSON.stringify,
      decode: decodeDragPayload
    })
  const orderedMembers = eagerReorder(members, (m) => m.story.number, draggedNumber, hoveredNumber)
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
            if (hasType(event)) {
              event.preventDefault()
              event.dataTransfer.dropEffect = 'move'
              if (draggedNumber !== null && draggedNumber !== story.number) {
                hover(story.number)
              }
            }
          }}
          onDrop={(event) => {
            const payload = decode(event)
            if (payload === null) return
            event.preventDefault()
            if (iteration.gh_state === 'closed') return
            const targetNumber =
              payload.storyNumber === story.number && hoveredNumber !== null
                ? hoveredNumber
                : story.number
            if (payload.storyNumber === targetNumber) return
            const originalIdx = members.findIndex((m) => m.story.number === targetNumber)
            if (originalIdx === -1) return
            const mutation = computeMemberDropMutation({
              draggedIterationNumber: payload.iterationNumber,
              draggedOldPosition: payload.oldPosition,
              targetIterationNumber: iteration.number,
              targetMemberPositions: members.map((m) => m.position),
              targetMemberIndex: originalIdx,
              targetMembersIncludeDragged: payload.iterationNumber === iteration.number
            })
            if (!mutation) return
            postLabelMutation(payload.storyNumber, mutation.add, mutation.remove)
            const addedLabel = mutation.add[0]
            const parsed = addedLabel ? parseParentIterationLabel(addedLabel) : null
            if (parsed) {
              setPending(payload.storyNumber, parsed.iterationNumber, parsed.position)
            }
            endDrag()
          }}
        >
          <button
            type="button"
            className="board-iteration-member-navigate board-story-navigate"
            data-test="board-iteration-member-button"
            aria-label={`open #${story.number} ${story.title}`}
            onClick={() => onOpen(story.number)}
            onKeyDown={(event) => {
              if (!event.ctrlKey || !event.shiftKey) return
              if (iteration.gh_state === 'closed') return
              const direction =
                event.key === 'ArrowUp' ? 'up' : event.key === 'ArrowDown' ? 'down' : null
              if (direction === null) return
              event.preventDefault()
              const originalIdx = members.findIndex((m) => m.story.number === story.number)
              if (originalIdx === -1) return
              if (direction === 'up' && originalIdx === 0) {
                onReorderAnnounce?.(`${story.title} is already at position 1 in ${iteration.title}`)
                return
              }
              if (direction === 'down' && originalIdx === members.length - 1) {
                onReorderAnnounce?.(
                  `${story.title} is already at last position in ${iteration.title}`
                )
                return
              }
              const targetMemberIndex = direction === 'up' ? originalIdx - 1 : originalIdx + 1
              const mutation = computeMemberDropMutation({
                draggedIterationNumber: iteration.number,
                draggedOldPosition: position,
                targetIterationNumber: iteration.number,
                targetMemberPositions: members.map((m) => m.position),
                targetMemberIndex,
                targetMembersIncludeDragged: true
              })
              if (!mutation) return
              postLabelMutation(story.number, mutation.add, mutation.remove)
              const addedLabel = mutation.add[0]
              const parsedKey = addedLabel ? parseParentIterationLabel(addedLabel) : null
              if (parsedKey) {
                setPending(story.number, parsedKey.iterationNumber, parsedKey.position)
              }
              const newIdx = direction === 'up' ? originalIdx - 1 : originalIdx + 1
              onReorderAnnounce?.(
                `Moved ${story.title} to position ${newIdx + 1} of ${members.length} in ${iteration.title}`
              )
            }}
          />
          {iteration.gh_state !== 'closed' ? (
            <button
              type="button"
              className="board-story-grip"
              data-test={STORY_GRIP_TEST_ID}
              draggable={true}
              aria-label={`Drag to reorder #${story.number}`}
              tabIndex={-1}
              onDragStart={(event) => {
                const payload: DragPayload = {
                  storyNumber: story.number,
                  iterationNumber: iteration.number,
                  oldPosition: position
                }
                startDrag(event, story.number, payload)
              }}
              onDragEnd={endDrag}
            >
              <GripIcon />
            </button>
          ) : null}
          <small className="board-story-number">#{story.number}</small>
          <StoryTypeGlyph type={storyTypeFromLabels(story.labels)} />
          <StoryBadge story={story} onOpenBlocker={onOpen} />
          {activeStateLabel(story) !== null && (
            <small className="board-iteration-member-state" data-state={story.state}>
              {activeStateLabel(story)}
            </small>
          )}
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
        </li>
      ))}
    </ul>
  )
}

export default MemberList
