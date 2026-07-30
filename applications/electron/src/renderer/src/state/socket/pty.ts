import { StateActionType } from '../actions'
import { encodeKeystroke } from '../ptyCodec'
import { translating } from './translating'

export const ptySubscribe = translating((action) =>
  action.type === StateActionType.PtySubscribeRequested
    ? { type: 'pty_subscribe', terminal_id: action.terminalId }
    : null
)

export const ptyInput = translating((action) =>
  action.type === StateActionType.PtyInputRequested
    ? {
        type: 'pty_input',
        terminal_id: action.terminalId,
        data: encodeKeystroke(action.keystroke)
      }
    : null
)

export const ptyResize = translating((action) =>
  action.type === StateActionType.PtyResizeRequested
    ? {
        type: 'pty_resize',
        terminal_id: action.terminalId,
        cols: action.cols,
        rows: action.rows
      }
    : null
)
