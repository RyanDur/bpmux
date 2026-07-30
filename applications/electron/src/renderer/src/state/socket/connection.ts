import type { ActionListener } from '../redux'
import type { StateSnapshot } from '../reducer'
import { StateActionType, type StateAction } from '../actions'
import type { SocketDoor } from './door'

export const connection =
  (door: SocketDoor): ActionListener<StateSnapshot, StateAction> =>
  (action, _before, _after, dispatch) => {
    switch (action.type) {
      case StateActionType.StartRequested:
        if (door.alreadyOn(action.config)) return
        door.adopt(action.config)
        door.connect(dispatch)
        return
      case StateActionType.Reset:
        door.reset()
        return
      default:
        return
    }
  }

export const frames =
  (door: SocketDoor): ActionListener<StateSnapshot, StateAction> =>
  (action, _before, _after, dispatch) => {
    if (action.type !== StateActionType.FrameReceived) return
    door.frame(action.raw, dispatch)
  }
