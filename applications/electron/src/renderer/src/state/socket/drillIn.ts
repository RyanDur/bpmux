import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import { StateActionType, type StateAction } from '../actions'
import type { SocketDoor } from './door'

export const commentsHydrate =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.CommentsHydrateRequested) return next(action)
    door.send({ type: 'comments_hydrate', story_id: action.storyId })
  }

export const timelineHydrate =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.TimelineHydrateRequested) return next(action)
    door.send({ type: 'timeline_hydrate', story_id: action.storyId })
  }
