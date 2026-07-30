import type { ActionListener } from './redux'
import type { StateSnapshot } from './reducer'
import { StateActionType, type StateAction } from './actions'
import { emitLiveMessage, emitPtyExit, emitPtyMode, emitPtyOutput } from './emitter'

export const emitListener: ActionListener<StateSnapshot, StateAction> = (action, before, after) => {
  switch (action.type) {
    case StateActionType.PtyOutputReceived:
      emitPtyOutput(action.terminalId, action.data)
      return
    case StateActionType.PtyModeReceived:
      emitPtyMode(action.terminalId, action.bracketedPaste)
      return
    case StateActionType.PtyExitReceived:
      emitPtyExit(action.terminalId)
      return
    case StateActionType.MessageReceived:
      if (after.messages !== before.messages) emitLiveMessage(action.message)
      return
    default:
      return
  }
}
