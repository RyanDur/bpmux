import type { Middleware } from '../../lib/redux'
import type { SessionSnapshot } from './reducer'
import { SessionActionType, type SessionAction } from './actions'
import { emitLiveMessage, emitPtyExit, emitPtyMode, emitPtyOutput } from './emitter'

export const emitMiddleware: Middleware<SessionSnapshot, SessionAction> =
  (api) => (next) => (action) => {
    if (action.type === SessionActionType.PtyOutputReceived) {
      next(action)
      emitPtyOutput(action.terminalId, action.data)
      return
    }
    if (action.type === SessionActionType.PtyModeReceived) {
      next(action)
      emitPtyMode(action.terminalId, action.bracketedPaste)
      return
    }
    if (action.type === SessionActionType.PtyExitReceived) {
      next(action)
      emitPtyExit(action.terminalId)
      return
    }
    if (action.type === SessionActionType.MessageReceived) {
      const before = api.getState().messages
      next(action)
      if (api.getState().messages !== before) emitLiveMessage(action.message)
      return
    }
    next(action)
  }
