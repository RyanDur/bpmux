import type { Middleware } from '../../lib/redux'
import type { StateSnapshot } from './reducer'
import { StateActionType, type StateAction } from './actions'
import { emitLiveMessage, emitPtyExit, emitPtyMode, emitPtyOutput } from './emitter'

export const emitMiddleware: Middleware<StateSnapshot, StateAction> =
  (api) => (next) => (action) => {
    if (action.type === StateActionType.PtyOutputReceived) {
      next(action)
      emitPtyOutput(action.terminalId, action.data)
      return
    }
    if (action.type === StateActionType.PtyModeReceived) {
      next(action)
      emitPtyMode(action.terminalId, action.bracketedPaste)
      return
    }
    if (action.type === StateActionType.PtyExitReceived) {
      next(action)
      emitPtyExit(action.terminalId)
      return
    }
    if (action.type === StateActionType.MessageReceived) {
      const before = api.getState().messages
      next(action)
      if (api.getState().messages !== before) emitLiveMessage(action.message)
      return
    }
    next(action)
  }
