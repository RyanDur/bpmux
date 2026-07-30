import { StateActionType } from '../actions'
import { encodeKeystroke } from '../ptyCodec'
import type { Translator } from './translating'

export const ptySubscribe: Translator = (action) =>
  action.type === StateActionType.PtySubscribeRequested
    ? { type: 'pty_subscribe', terminal_id: action.terminalId }
    : null

export const ptyInput: Translator = (action) =>
  action.type === StateActionType.PtyInputRequested
    ? {
        type: 'pty_input',
        terminal_id: action.terminalId,
        data: encodeKeystroke(action.keystroke)
      }
    : null

export const ptyResize: Translator = (action) =>
  action.type === StateActionType.PtyResizeRequested
    ? {
        type: 'pty_resize',
        terminal_id: action.terminalId,
        cols: action.cols,
        rows: action.rows
      }
    : null
