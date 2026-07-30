import { DrillInActionType } from './actions'
import type { StateAction } from '../../actions'
import type { CommentWire, TimelineEventWire } from '@transport/types'

export type DrillInState = {
  commentsByStory: Readonly<Record<number, CommentWire[]>>
  timelineByStory: Readonly<Record<number, TimelineEventWire[]>>
}

export const initialDrillIn: DrillInState = {
  commentsByStory: {},
  timelineByStory: {}
}

export const drillInFold = (
  state: DrillInState = initialDrillIn,
  action: StateAction
): DrillInState => {
  switch (action.type) {
    case DrillInActionType.CommentsHydrated:
      return {
        ...state,
        commentsByStory: { ...state.commentsByStory, [action.storyId]: action.comments }
      }
    case DrillInActionType.TimelineHydrated:
      return {
        ...state,
        timelineByStory: { ...state.timelineByStory, [action.storyId]: action.events }
      }
    default:
      return state
  }
}
