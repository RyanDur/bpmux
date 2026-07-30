import { StateActionType } from '../actions'
import type { Translator } from './translating'

export const commentsHydrate: Translator = (action) =>
  action.type === StateActionType.CommentsHydrateRequested
    ? { type: 'comments_hydrate', story_id: action.storyId }
    : null

export const timelineHydrate: Translator = (action) =>
  action.type === StateActionType.TimelineHydrateRequested
    ? { type: 'timeline_hydrate', story_id: action.storyId }
    : null
