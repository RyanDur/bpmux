import type { ActionListener } from '../redux'
import type { StateSnapshot } from '../reducer'
import type { StateAction } from '../actions'
import type { ClientMessage } from '@transport/types'
import type { SocketDoor } from './door'

export type Translator = (action: StateAction) => ClientMessage | null

export const translating =
  (door: SocketDoor, translators: Translator[]): ActionListener<StateSnapshot, StateAction> =>
  (action) => {
    for (const frameFor of translators) {
      const frame = frameFor(action)
      if (frame !== null) {
        door.send(frame)
        return
      }
    }
  }
