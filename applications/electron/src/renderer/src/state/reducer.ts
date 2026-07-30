import { type ConnectionState, connectionFold, initialConnection } from './domains/connection/fold'
import { type ChatState, chatFold, initialChat } from './domains/chat/fold'
import { type DrillInState, drillInFold, initialDrillIn } from './domains/drillIn/fold'
import { type BoardState, boardFold, initialBoard } from './domains/board/fold'
import { type MirrorState, initialMirror, mirrorFold } from './domains/mirror/fold'
import type { StateAction } from './actions'

export type StateSnapshot = ConnectionState &
  ChatState &
  DrillInState &
  BoardState & { mirror: MirrorState }

export const emptySnapshot = (url: string | undefined): StateSnapshot => ({
  ...initialConnection,
  status: url ? 'connecting' : 'disconnected',
  ...initialChat,
  ...initialDrillIn,
  ...initialBoard,
  mirror: initialMirror
})

export const stateReducer = (state: StateSnapshot, action: StateAction): StateSnapshot => {
  const patches = [
    connectionFold(state, action),
    chatFold(state, action),
    drillInFold(state, action),
    boardFold(state, action),
    mirrorFold(state, action)
  ].filter((folded) => folded !== state)
  return patches.length === 0 ? state : Object.assign({}, state, ...patches)
}
