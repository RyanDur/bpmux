import { StateActionType } from '../actions'
import { translating } from './translating'

export const commentsHydrate = translating((action) =>
  action.type === StateActionType.CommentsHydrateRequested
    ? { type: 'comments_hydrate', story_id: action.storyId }
    : null
)

export const timelineHydrate = translating((action) =>
  action.type === StateActionType.TimelineHydrateRequested
    ? { type: 'timeline_hydrate', story_id: action.storyId }
    : null
)
