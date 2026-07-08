import { DragGrip } from './DragGrip'
import { ReworkGlyph } from './ReworkGlyph'
import { SortableList } from './SortableList'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { storyNavigateLabel } from './storyNavigateLabel'
import {
  columnForIn,
  groupOrderOrFallback,
  middleColumnIds,
  storyTypeFromLabels,
  ICEBOX_COLUMN,
  type GroupSpec,
  type Story
} from './types'
import { type DragPayload } from './iterationMembers'
import { ITERATION_MEMBER_DRAG } from './sortableDragConfigs'
import { useDispatch } from '../transport/useDispatch'
import { useBoardColumns } from '../transport/state'
import { addStoryToIterationRequested } from '../transport/state/actions'
import type { BoardIterationData } from './selectIterations'
import type { IterationMember } from './useIterationMembers'

const activeStateLabel = (story: Story, groupOrder: GroupSpec[]): string | null => {
  if (story.state === ICEBOX_COLUMN) return null
  const column = columnForIn(middleColumnIds(groupOrder))(story.state)
  if (column === ICEBOX_COLUMN) return null
  return groupOrder.find((group) => group.state === column)?.label ?? null
}

interface Props {
  iteration: BoardIterationData
  members: IterationMember[]
  setPending: (storyId: number, iterationId: number, position: number) => void
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
  const dispatch = useDispatch()
  const groupOrder = groupOrderOrFallback(useBoardColumns())
  return (
    <SortableList<DragPayload, IterationMember>
      items={members}
      keyOf={(m) => m.story.id}
      rowKey={(m) => m.story.id}
      config={ITERATION_MEMBER_DRAG}
      buildPayload={(m) => ({
        storyId: m.story.id,
        iterationId: iteration.id,
        oldPosition: m.position
      })}
      disabled={iteration.complete}
      onReorder={(payload, targetKey) => {
        const originalIdx = members.findIndex((m) => m.story.id === targetKey)
        if (originalIdx === -1) return
        const position = originalIdx + 1
        dispatch(addStoryToIterationRequested(iteration.id, payload.storyId, position))
        setPending(payload.storyId, iteration.id, position)
      }}
      ulClassName="board-iteration-body"
      renderRow={({ story }, { rowDragProps, gripDragProps, dragging }) => (
        <li
          className="board-iteration-member"
          data-test={`board-story-${story.id}`}
          data-iteration-member={story.id}
          data-dragging={dragging ? 'true' : undefined}
          {...rowDragProps}
        >
          <button
            type="button"
            className="board-iteration-member-navigate board-story-navigate"
            aria-label={storyNavigateLabel(story)}
            onClick={() => onOpen(story.id)}
            onKeyDown={(event) => {
              if (!event.ctrlKey || !event.shiftKey) return
              if (iteration.complete) return
              const direction =
                event.key === 'ArrowUp' ? 'up' : event.key === 'ArrowDown' ? 'down' : null
              if (direction === null) return
              event.preventDefault()
              const originalIdx = members.findIndex((m) => m.story.id === story.id)
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
              const newPosition = targetMemberIndex + 1
              dispatch(addStoryToIterationRequested(iteration.id, story.id, newPosition))
              setPending(story.id, iteration.id, newPosition)
              onReorderAnnounce?.(
                `Moved ${story.title} to position ${newPosition} of ${members.length} in ${iteration.title}`
              )
            }}
          />
          {!iteration.complete ? (
            <DragGrip storyId={story.id} gripDragProps={gripDragProps} />
          ) : null}
          <small className="board-story-number">#{story.id}</small>
          <StoryTypeGlyph type={storyTypeFromLabels(story.labels)} />
          <StoryBadge story={story} />
          {activeStateLabel(story, groupOrder) !== null && (
            <small className="board-iteration-member-state" data-state={story.state}>
              {activeStateLabel(story, groupOrder)}
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
