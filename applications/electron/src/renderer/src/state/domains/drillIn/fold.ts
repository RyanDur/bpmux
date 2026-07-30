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

export const drillInFold = (slice: DrillInSlice, action: StateAction): DrillInSlice | null => {
  switch (action.type) {
    case StateActionType.CommentsHydrated: {
      const commentsByStory = new Map(slice.commentsByStory)
      commentsByStory.set(action.storyId, action.comments)
      return { ...slice, commentsByStory }
    }
    case StateActionType.TimelineHydrated: {
      const timelineByStory = new Map(slice.timelineByStory)
      timelineByStory.set(action.storyId, action.events)
      return { ...slice, timelineByStory }
    }
    default:
      return null
  }
}
