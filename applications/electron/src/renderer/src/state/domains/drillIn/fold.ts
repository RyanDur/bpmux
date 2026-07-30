import { caseOf } from '@ryandur/sand'
import { StateActionType, type StateAction } from '../../actions'
import type { CommentWire, TimelineEventWire } from '@transport/types'

export type DrillInState = {
  commentsByStory: Readonly<Record<number, CommentWire[]>>
  timelineByStory: Readonly<Record<number, TimelineEventWire[]>>
}

export const initialDrillIn: DrillInState = {
  commentsByStory: {},
  timelineByStory: {}
}

const onActionType = caseOf('type')

export const drillInFold = (
  state: DrillInState = initialDrillIn,
  action: StateAction
): DrillInState =>
  onActionType<StateAction, DrillInState>(action, {
    [StateActionType.CommentsHydrated]: ({ storyId, comments }) => ({
      ...state,
      commentsByStory: { ...state.commentsByStory, [storyId]: comments }
    }),
    [StateActionType.TimelineHydrated]: ({ storyId, events }) => ({
      ...state,
      timelineByStory: { ...state.timelineByStory, [storyId]: events }
    })
  }).orElse(state)
