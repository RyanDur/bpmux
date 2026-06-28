import { DragGrip } from './DragGrip'
import { ReworkGlyph } from './ReworkGlyph'
import { SortableList } from './SortableList'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { computeMemberDropMutation } from './iterationPosition'
import { parseParentIterationLabel } from './labelSchema'
import { storyNavigateLabel } from './storyNavigateLabel'
import { GROUP_ORDER, columnFor, storyTypeFromLabels, type BoardStory } from './types'
import { type DragPayload, postLabelMutation } from './iterationMembers'
import { ITERATION_MEMBER_DRAG } from './sortableDragConfigs'
import type { BoardIterationData } from './selectIterations'
import type { IterationMember } from './useIterationMembers'

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
  iteration: BoardIterationData
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
  return (
    <SortableList<DragPayload, IterationMember>
      items={members}
      keyOf={(m) => m.story.number}
      rowKey={(m) => m.story.number}
      config={ITERATION_MEMBER_DRAG}
      buildPayload={(m) => ({
        storyNumber: m.story.number,
        iterationNumber: iteration.number,
        oldPosition: m.position
      })}
      disabled={iteration.complete}
      onReorder={(payload, targetKey) => {
        const originalIdx = members.findIndex((m) => m.story.number === targetKey)
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
      }}
      ulClassName="board-iteration-body"
      renderRow={({ story, position }, { rowDragProps, gripDragProps, dragging }) => (
        <li
          className="board-iteration-member"
          data-test={`board-story-${story.number}`}
          data-iteration-member={story.number}
          data-dragging={dragging ? 'true' : undefined}
          {...rowDragProps}
        >
          <button
            type="button"
            className="board-iteration-member-navigate board-story-navigate"
            aria-label={storyNavigateLabel(story)}
            onClick={() => onOpen(story.number)}
            onKeyDown={(event) => {
              if (!event.ctrlKey || !event.shiftKey) return
              if (iteration.complete) return
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
          {!iteration.complete ? (
            <DragGrip storyNumber={story.number} gripDragProps={gripDragProps} />
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
      )}
    />
  )
}

export default MemberList
