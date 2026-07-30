import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import type { StateAction } from '../actions'
import type { ClientMessage } from '@transport/types'
import type { SocketDoor } from './door'

export const translating =
  (frameFor: (action: StateAction) => ClientMessage | null) =>
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  (_storeApi) =>
  (next) =>
  (action) => {
    const frame = frameFor(action)
    if (frame === null) return next(action)
    door.send(frame)
  }
