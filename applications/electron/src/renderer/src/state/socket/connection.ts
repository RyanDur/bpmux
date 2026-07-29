import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import { StateActionType, type StateAction } from '../actions'
import type { SocketDoor } from './door'

export const connection =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  (api) =>
  (next) =>
  (action) => {
    switch (action.type) {
      case StateActionType.StartRequested:
        if (door.alreadyOn(action.config)) return
        door.adopt(action.config)
        next(action)
        door.connect((dispatched) => api.dispatch(dispatched))
        return
      case StateActionType.Reset:
        door.reset()
        next(action)
        return
      default:
        next(action)
    }
  }

export const frames =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  (api) =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.FrameReceived) return next(action)
    door.frame(action.raw, (dispatched) => api.dispatch(dispatched))
  }
