import type { Middleware } from '../redux'
import type { StateSnapshot } from '../reducer'
import { StateActionType, type StateAction } from '../actions'
import { encodeKeystroke } from '../ptyCodec'
import type { SocketDoor } from './door'

export const ptySubscribe =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.PtySubscribeRequested) return next(action)
    door.send({ type: 'pty_subscribe', terminal_id: action.terminalId })
  }

export const ptyInput =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.PtyInputRequested) return next(action)
    door.send({
      type: 'pty_input',
      terminal_id: action.terminalId,
      data: encodeKeystroke(action.keystroke)
    })
  }

export const ptyResize =
  (door: SocketDoor): Middleware<StateSnapshot, StateAction> =>
  () =>
  (next) =>
  (action) => {
    if (action.type !== StateActionType.PtyResizeRequested) return next(action)
    door.send({
      type: 'pty_resize',
      terminal_id: action.terminalId,
      cols: action.cols,
      rows: action.rows
    })
  }
