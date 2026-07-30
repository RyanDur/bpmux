import { caseOf } from '@ryandur/sand'
import { StateActionType, type StateAction } from '../../actions'
import type { CommentWire, TimelineEventWire } from '@transport/types'

export type DrillInSlice = {
  commentsByStory: ReadonlyMap<number, CommentWire[]>
  timelineByStory: ReadonlyMap<number, TimelineEventWire[]>
}

export const emptyDrillIn = (): DrillInSlice => ({
  commentsByStory: new Map(),
  timelineByStory: new Map()
})

const onActionType = caseOf('type')

export const drillInFold = (slice: DrillInSlice, action: StateAction): DrillInSlice | null =>
  onActionType<StateAction, DrillInSlice | null>(action, {
    [StateActionType.CommentsHydrated]: ({ storyId, comments }) => {
      const commentsByStory = new Map(slice.commentsByStory)
      commentsByStory.set(storyId, comments)
      return { ...slice, commentsByStory }
    },
    [StateActionType.TimelineHydrated]: ({ storyId, events }) => {
      const timelineByStory = new Map(slice.timelineByStory)
      timelineByStory.set(storyId, events)
      return { ...slice, timelineByStory }
    }
  }).orNull()
